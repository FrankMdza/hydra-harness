#!/usr/bin/env python3
"""Round one on a rented T4: QLoRA on Ministral 3B, scored before and after on the same tests.

Two scores, both on the GPU, both greedy:
- next call: for every tool call in eval.jsonl, the model sees the trajectory up to it and must
  name the same tool (and, for Read, the same file);
- bake-off: map 111's ten single-turn tasks, which the base model passes 10 of 10 through Ollama,
  so a drop here means training broke tool calling.

Loss falls on the assistant turns only. Usage on the VM: python3 round1.py DATA_DIR OUT_DIR
"""
import json
import re
import sys
import time

import torch
from peft import LoraConfig, get_peft_model, prepare_model_for_kbit_training
from transformers import AutoModelForImageTextToText, AutoTokenizer, BitsAndBytesConfig

BASE = "mistralai/Ministral-3-3B-Instruct-2512-BF16"
MAX_TOKENS = 4096
TRAIN_TOKENS = 2048  # 4096 ran a T4 out of memory: the fp32 logits alone are 2 GB
EPOCHS = 3
LR = 2e-4

BAKEOFF = [
    ("Show me what is in src/run.rs.", "Read", "src/run.rs"),
    ("Run the test suite with cargo.", "Bash", "cargo test"),
    ("Create notes.md containing the line 'hello'.", "Write", "notes.md"),
    ("In README.md, change the word 'macOS' to 'macOS and Linux'.", "Edit", "README.md"),
    ("How many lines does Cargo.toml have? Use a shell command.", "Bash", "wc"),
    ("Open the file docs/harness-spec.md.", "Read", "docs/harness-spec.md"),
    ("Show the last three commits.", "Bash", "git log"),
    ("Write a file hi.txt that says 'hi there'.", "Write", "hi.txt"),
    ("In src/lib.rs replace 'pub mod run;' with 'pub mod run; pub mod bench;'.", "Edit", "src/lib.rs"),
    ("List the files in the tests directory.", "Bash", "ls"),
]


def render(tok, messages, tools, gen):
    """Token ids straight from the template; re-encoding its text loses the control tokens."""
    ids = tok.apply_chat_template(messages, tools=tools, tokenize=True, add_generation_prompt=gen)
    ids = ids["input_ids"] if hasattr(ids, "keys") else ids
    return list(ids[0]) if ids and isinstance(ids[0], list) else list(ids)


def encode(tok, tok_train, messages, tools):
    """Token ids and labels, labels set only on the assistant turns.

    `tok` validates for serving and refuses a last assistant turn; `tok_train` is the same
    tokenizer in finetuning mode, which wants one, so each side renders what it accepts."""
    labels, done = [], []
    for i in range(1, len(messages) + 1):
        if messages[i - 1]["role"] != "assistant":
            continue
        before = render(tok, messages[:i - 1], tools, True)
        after = render(tok_train, messages[:i], tools, False)
        if after[:len(before)] != before or before[:len(done)] != done:
            raise ValueError("the chat template is not prefix-stable")
        labels += [-100] * (len(before) - len(done)) + after[len(before):]
        done = after
    return done[:TRAIN_TOKENS], labels[:TRAIN_TOKENS]


CALL = re.compile(r"\[TOOL_CALLS\]\s*(\w+)\s*\[ARGS\]\s*(\{.*\})", re.S)
CALL_LIST = re.compile(r"\[TOOL_CALLS\]\s*(\[.*\])", re.S)


def parse(text):
    m = CALL.search(text)
    if m:
        try:
            return m.group(1), json.loads(m.group(2))
        except json.JSONDecodeError:
            return m.group(1), None
    m = CALL_LIST.search(text)
    if m:
        try:
            call = json.loads(m.group(1))[0]
            return call["name"], call.get("arguments")
        except Exception:
            return None, None
    return None, None


@torch.no_grad()
def generate(model, tok, messages, tools):
    prompt = render(tok, messages, tools, True)
    if len(prompt) > MAX_TOKENS:
        return None
    ids = torch.tensor([prompt], device=model.device)
    out = model.generate(input_ids=ids, max_new_tokens=160, do_sample=False)
    return tok.decode(out[0, ids.shape[1]:], skip_special_tokens=False)


def score(model, tok, tools, eval_rows, system):
    model.eval()
    nxt = []
    for row in eval_rows:
        msgs = row["messages"]
        for i, m in enumerate(msgs):
            if "tool_calls" not in m or len(nxt) >= 40:
                continue
            want = m["tool_calls"][0]["function"]
            text = generate(model, tok, msgs[:i], tools)
            if text is None:
                continue
            name, args = parse(text)
            ok = name == want["name"] and (want["name"] != "Read" or (args or {}).get("file_path") == want["arguments"]["file_path"])
            nxt.append({"want": want["name"], "got": name, "ok": bool(ok)})
    bake = []
    for task, want, needle in BAKEOFF:
        text = generate(model, tok, [{"role": "system", "content": system}, {"role": "user", "content": task}], tools)
        name, args = parse(text)
        bake.append({"task": task, "ok": name == want and needle in json.dumps(args or {}), "got": name,
                     "raw": None if name else text[:200]})
    return {"next_call": sum(r["ok"] for r in nxt), "next_of": len(nxt),
            "bakeoff": sum(r["ok"] for r in bake), "bakeoff_of": len(bake), "rows": nxt, "bake_rows": bake}


def main(data, out):
    tools = json.load(open(f"{data}/tools.json"))
    train = [json.loads(l) for l in open(f"{data}/train.jsonl")]
    evals = [json.loads(l) for l in open(f"{data}/eval.jsonl")]
    system = train[0]["messages"][0]["content"]
    tok = AutoTokenizer.from_pretrained(BASE)
    tok_train = AutoTokenizer.from_pretrained(BASE, mode="finetuning")
    quant = BitsAndBytesConfig(load_in_4bit=True, bnb_4bit_quant_type="nf4",
                               bnb_4bit_compute_dtype=torch.float16, bnb_4bit_use_double_quant=True)
    model = AutoModelForImageTextToText.from_pretrained(BASE, quantization_config=quant,
                                                        torch_dtype=torch.float16, device_map="cuda:0")
    report = {"base": BASE, "train_runs": len(train), "eval_runs": len(evals)}

    t = time.time()
    report["before"] = score(model, tok, tools, evals, system)
    report["before_secs"] = round(time.time() - t)
    print(json.dumps({k: v for k, v in report["before"].items() if not k.endswith("rows")}), flush=True)

    model = prepare_model_for_kbit_training(model, use_gradient_checkpointing=True)
    lora = LoraConfig(r=16, lora_alpha=32, lora_dropout=0.05, task_type="CAUSAL_LM",
                      target_modules=r".*language_model.*\.(q_proj|k_proj|v_proj|o_proj|gate_proj|up_proj|down_proj)")
    model = get_peft_model(model, lora)
    model.print_trainable_parameters()
    batches = [b for b in (encode(tok, tok_train, r["messages"], tools) for r in train)
               if any(x != -100 for x in b[1])]  # a row cut before its first answer has no loss
    report["train_tokens"] = sum(len(b[0]) for b in batches)
    report["train_label_tokens"] = sum(sum(x != -100 for x in b[1]) for b in batches)
    opt = torch.optim.AdamW([p for p in model.parameters() if p.requires_grad], lr=LR)
    scaler = torch.amp.GradScaler("cuda")
    model.train()
    losses, t = [], time.time()
    for epoch in range(EPOCHS):
        for step, (ids, labels) in enumerate(batches):
            x = torch.tensor([ids], device=model.device)
            y = torch.tensor([labels], device=model.device)
            with torch.autocast("cuda", dtype=torch.float16):
                loss = model(input_ids=x, labels=y).loss
            if not torch.isfinite(loss):
                losses.append(None); opt.zero_grad(); continue
            scaler.scale(loss).backward()
            scaler.unscale_(opt)
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            scaler.step(opt); scaler.update(); opt.zero_grad()
            losses.append(round(loss.item(), 4))
        done = [l for l in losses[-len(batches):] if l is not None]
        print(json.dumps({"epoch": epoch, "rows": len(batches), "finite": len(done),
                          "mean_loss": sum(done) / max(1, len(done))}), flush=True)
    report["train_secs"] = round(time.time() - t)
    report["losses"] = losses
    model.save_pretrained(f"{out}/adapter")

    t = time.time()
    report["after"] = score(model, tok, tools, evals, system)
    report["after_secs"] = round(time.time() - t)
    print(json.dumps({k: v for k, v in report["after"].items() if not k.endswith("rows")}), flush=True)
    json.dump(report, open(f"{out}/report.json", "w"), indent=1)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])

//! `record_to_otlp.py`: a record, or a directory of them, becomes an OTLP JSON trace.

use std::fs::{self, DirBuilder};
use std::io::Write;
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Result, anyhow, bail};
use hydra_pyjson::file::{os_error, read_text};
use hydra_pyjson::json::{self, Dict, PyStr, Value};
use hydra_pyjson::py;
use hydra_records::isotime;
use hydra_records::records;
use hydra_records::redact;
use sha2::{Digest, Sha256};

const TASK_BUDGET: usize = 2000;
const EPOCH_MICROS: i128 = 719_162 * 86_400_000_000;

fn json_err(error: json::ParseError, message: String) -> anyhow::Error {
    let class = match error {
        json::ParseError::Decode => "json.decoder.JSONDecodeError",
        other => other.class(),
    };
    anyhow!("{class}: {message}")
}

fn io_err(err: &std::io::Error, path: &Path) -> anyhow::Error {
    anyhow!("{}: {}", os_error(err), path.display())
}

fn os_error_text(err: &std::io::Error, path: &str) -> String {
    let said = err.to_string();
    let said = match said.rfind(" (os error ") {
        Some(at) => &said[..at],
        None => &said,
    };
    match err.raw_os_error() {
        Some(errno) => format!("[Errno {errno}] {said}: {}", py::repr(&Value::text(path))),
        None => said.to_string(),
    }
}

fn return_code(status: &std::process::ExitStatus) -> i32 {
    status
        .code()
        .unwrap_or_else(|| -status.signal().unwrap_or(0))
}

fn signal_name(signal: i32) -> Option<&'static str> {
    Some(match signal {
        1 => "SIGHUP",
        2 => "SIGINT",
        3 => "SIGQUIT",
        6 => "SIGABRT",
        9 => "SIGKILL",
        11 => "SIGSEGV",
        13 => "SIGPIPE",
        14 => "SIGALRM",
        15 => "SIGTERM",
        _ => return None,
    })
}

fn no_program() -> anyhow::Error {
    let path = std::env::var("PATH").unwrap_or_else(|_| "/bin:/usr/bin".into());
    let any = path.split(':').any(|dir| {
        let dir = if dir.is_empty() { "." } else { dir };
        Path::new(dir).is_dir()
    });
    if any {
        anyhow!("PermissionError: [Errno 13] Permission denied: ''")
    } else {
        anyhow!("FileNotFoundError: [Errno 2] No such file or directory: ''")
    }
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<TempDir> {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let mut places: Vec<PathBuf> = ["TMPDIR", "TEMP", "TMP"]
            .iter()
            .filter_map(std::env::var_os)
            .map(PathBuf::from)
            .collect();
        places.extend(["/tmp", "/var/tmp", "/usr/tmp", "."].map(PathBuf::from));
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.subsec_nanos());
        let name = format!(
            "hydra-otel-{}-{}-{nanos}",
            std::process::id(),
            COUNT.fetch_add(1, Ordering::Relaxed)
        );
        for base in &places {
            let path = base.join(&name);
            if DirBuilder::new().mode(0o700).create(&path).is_ok() {
                return Ok(TempDir(path));
            }
        }
        let tried: Vec<Value> = places
            .iter()
            .map(|place| Value::text(&place.display().to_string()))
            .collect();
        bail!(
            "FileNotFoundError: [Errno 2] No usable temporary directory found in {}",
            py::repr(&Value::List(tried))
        )
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn split_lines(data: &[u8]) -> Vec<&[u8]> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < data.len() {
        let end = match data[i] {
            b'\n' => i + 1,
            b'\r' if data.get(i + 1) == Some(&b'\n') => i + 2,
            b'\r' => i + 1,
            _ => {
                i += 1;
                continue;
            }
        };
        lines.push(&data[start..end]);
        start = end;
        i = end;
    }
    if start < data.len() {
        lines.push(&data[start..]);
    }
    lines
}

fn is_bytes_space(b: u8) -> bool {
    matches!(b, b'\t' | b'\n' | 0x0b | b'\x0c' | b'\r' | b' ')
}

fn strip_bytes(line: &[u8]) -> &[u8] {
    let start = line
        .iter()
        .position(|b| !is_bytes_space(*b))
        .unwrap_or(line.len());
    let end = line
        .iter()
        .rposition(|b| !is_bytes_space(*b))
        .map_or(0, |i| i + 1);
    if start >= end { &[] } else { &line[start..end] }
}

fn read_rows(path: &Path) -> Result<Vec<Value>> {
    let data = fs::read(path).map_err(|err| io_err(&err, path))?;
    let mut rows = Vec::new();
    for line in split_lines(&data) {
        if !line.ends_with(b"\n") {
            break;
        }
        if strip_bytes(line).is_empty() {
            continue;
        }
        match json::loads_bytes_explained(line) {
            Ok(value) => rows.push(value),
            Err((error, message)) => return Err(json_err(error, message)),
        }
    }
    let hydra = match rows.first() {
        Some(first) => py::get(first, "hydra", Value::Dict(Dict::new()))?,
        None => bail!("ValueError: record must begin with run_start"),
    };
    let typ = py::get(&hydra, "type", Value::Null)?;
    if !py::eq_str(&typ, "run_start") {
        bail!("ValueError: record must begin with run_start");
    }
    Ok(rows)
}

fn replace_z(text: &PyStr) -> PyStr {
    let mut out = PyStr::new();
    for code in text.code_points() {
        if *code == u32::from('Z') {
            out.push_str("+00:00");
        } else {
            out.push_code(*code);
        }
    }
    out
}

fn nano(timestamp: &Value) -> Result<String> {
    let Value::Str(text) = timestamp else {
        bail!(
            "AttributeError: '{}' object has no attribute 'replace'",
            py::type_name(timestamp)
        );
    };
    let replaced = replace_z(text);
    let Some(dt) = isotime::fromisoformat(&replaced) else {
        bail!(
            "ValueError: Invalid isoformat string: {}",
            py::repr(&Value::Str(replaced))
        );
    };
    let Some(instant) = dt.instant() else {
        bail!("TypeError: can't subtract offset-naive and offset-aware datetimes");
    };
    Ok(((instant - EPOCH_MICROS) * 1000).to_string())
}

fn identifier(text: &PyStr, length: usize) -> Result<String> {
    let Some(utf8) = text.to_utf8() else {
        bail!("UnicodeEncodeError: 'utf-8' codec can't encode a surrogate in an identifier");
    };
    let hex: String = Sha256::digest(utf8.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(hex[..length].to_string())
}

fn concat_str(left: &str, right: &Value) -> Result<PyStr> {
    let Value::Str(right) = right else {
        bail!(
            "TypeError: can only concatenate str (not \"{}\") to str",
            py::type_name(right)
        );
    };
    let mut out = PyStr::from(left);
    out.push(right);
    Ok(out)
}

fn span_id(run_id: &Value) -> Result<String> {
    identifier(&concat_str("run:", run_id)?, 16)
}

fn attributes(values: &Dict) -> Result<Value> {
    let mut result = Vec::new();
    for (key, value) in values.iter() {
        if matches!(value, Value::Null) {
            continue;
        }
        let encoded = match value {
            Value::Bool(flag) => {
                let mut encoded = Dict::new();
                encoded.set("boolValue", Value::Bool(*flag));
                Value::Dict(encoded)
            }
            Value::Int(n) => {
                let mut encoded = Dict::new();
                encoded.set("intValue", Value::text(&n.to_string()));
                Value::Dict(encoded)
            }
            Value::BigInt(digits) => {
                let mut encoded = Dict::new();
                encoded.set("intValue", Value::text(digits));
                Value::Dict(encoded)
            }
            other => {
                let mut encoded = Dict::new();
                encoded.set(
                    "stringValue",
                    Value::Str(redact::redact_string(&py::str(other))),
                );
                Value::Dict(encoded)
            }
        };
        let mut item = Dict::new();
        item.set("key", Value::Str(redact::redact_string(key)));
        item.set("value", encoded);
        result.push(Value::Dict(item));
    }
    Ok(Value::List(result))
}

fn put(map: &mut Vec<(Value, Value)>, key: Value, val: Value) -> Result<()> {
    py::hashable(&key)?;
    if let Some(slot) = map.iter_mut().find(|(k, _)| py::same_key(k, &key)) {
        slot.1 = val;
    } else {
        map.push((key, val));
    }
    Ok(())
}

fn get_eq<'a>(map: &'a [(Value, Value)], key: &Value) -> Option<&'a Value> {
    map.iter()
        .find(|(k, _)| py::same_key(k, key))
        .map(|(_, v)| v)
}

fn get_times<'a>(times: &'a [(Value, String)], key: &Value) -> Result<&'a str> {
    get_eq_str(times, key).ok_or_else(|| anyhow!("KeyError: {}", py::repr(key)))
}

fn get_eq_str<'a>(map: &'a [(Value, String)], key: &Value) -> Option<&'a str> {
    map.iter()
        .find(|(k, _)| py::same_key(k, key))
        .map(|(_, v)| v.as_str())
}

fn put_time(map: &mut Vec<(Value, String)>, key: Value, val: String) -> Result<()> {
    py::hashable(&key)?;
    if let Some(slot) = map.iter_mut().find(|(k, _)| py::same_key(k, &key)) {
        slot.1 = val;
    } else {
        map.push((key, val));
    }
    Ok(())
}

fn seen_has(seen: &[Value], key: &Value) -> Result<bool> {
    py::hashable(key)?;
    Ok(seen.iter().any(|item| py::same_key(item, key)))
}

fn session_id(first: &Value, directory: &Path) -> Result<String> {
    let mut current = first.clone();
    let run_id = py::item(&current, "run_id")?.clone();
    py::hashable(&run_id)?;
    let mut seen = vec![run_id];
    loop {
        let parent = py::get(&current, "parent_run_id", Value::Null)?;
        if !py::truthy(&parent) {
            break;
        }
        if seen_has(&seen, &parent)? {
            bail!("ValueError: cycle in parent_run_id");
        }
        seen.push(parent.clone());
        let Value::Str(parent_s) = &parent else {
            bail!(
                "TypeError: can only concatenate str (not \"{}\") to str",
                py::type_name(&parent)
            );
        };
        let Some(name) = parent_s.to_utf8() else {
            bail!("UnicodeEncodeError: 'utf-8' codec can't encode a surrogate in parent_run_id");
        };
        let path = directory.join(format!("{name}.jsonl"));
        if !path.is_file() {
            return Ok(name);
        }
        let rows = read_rows(&path)?;
        current = rows
            .into_iter()
            .next()
            .expect("read_rows requires a run_start");
    }
    let id = py::item(&current, "run_id")?;
    let Value::Str(text) = id else {
        bail!(
            "TypeError: can only concatenate str (not \"{}\") to str",
            py::type_name(id)
        );
    };
    text.to_utf8().ok_or_else(|| {
        anyhow!("UnicodeEncodeError: 'utf-8' codec can't encode a surrogate in run_id")
    })
}

fn hydra_bin() -> String {
    std::env::var("HYDRA_BIN").unwrap_or_else(|_| "hydra".into())
}

fn normalized(rows: &[Value], run_id: &Value) -> Result<Value> {
    let Value::Str(run_id) = run_id else {
        bail!(
            "TypeError: can only concatenate str (not \"{}\") to str",
            py::type_name(run_id)
        );
    };
    let Some(id) = run_id.to_utf8() else {
        bail!("UnicodeEncodeError: 'utf-8' codec can't encode a surrogate in a run id");
    };
    if id.contains('\0') {
        bail!("ValueError: embedded null byte");
    }
    let tmp = TempDir::new()?;
    let runs = tmp.0.join("runs");
    fs::create_dir(&runs).map_err(|err| io_err(&err, &runs))?;
    let path = runs.join(format!("{id}.jsonl"));
    let mut text = String::new();
    for row in rows {
        text.push_str(&json::dumps_default(row));
        text.push('\n');
    }
    fs::write(&path, text).map_err(|err| io_err(&err, &path))?;
    let hydra = hydra_bin();
    if hydra.is_empty() {
        return Err(no_program());
    }
    if hydra.contains('\0') {
        bail!("ValueError: embedded null byte");
    }
    let done = Command::new(&hydra)
        .arg("show")
        .arg(&id)
        .arg("--events")
        .env("HYDRA_ROOT", &tmp.0)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| anyhow!("{}: {}", os_error(&err), os_error_text(&err, &hydra)))?;
    drop(tmp);
    if !done.status.success() {
        let argv: Vec<Value> = [hydra.as_str(), "show", &id, "--events"]
            .iter()
            .map(|arg| Value::text(arg))
            .collect();
        let argv = py::repr(&Value::List(argv));
        let code = return_code(&done.status);
        if code >= 0 {
            bail!(
                "subprocess.CalledProcessError: Command '{argv}' returned non-zero exit status {code}."
            );
        }
        match signal_name(-code) {
            Some(name) => bail!(
                "subprocess.CalledProcessError: Command '{argv}' died with <Signals.{name}: {}>.",
                -code
            ),
            None => bail!(
                "subprocess.CalledProcessError: Command '{argv}' died with unknown signal {}.",
                -code
            ),
        }
    }
    let stdout = String::from_utf8(done.stdout).map_err(|_| {
        anyhow!("UnicodeDecodeError: 'utf-8' codec can't decode what hydra show printed")
    })?;
    match json::loads_explained(&PyStr::from(stdout)) {
        Ok(value) => Ok(value),
        Err((error, message)) => Err(json_err(error, message)),
    }
}

fn span(
    trace_id: &str,
    name: &PyStr,
    sid: String,
    begin: String,
    finish: String,
    attrs: &Dict,
    parent: Option<String>,
) -> Result<Value> {
    let mut result = Dict::new();
    result.set("traceId", Value::text(trace_id));
    result.set("spanId", Value::text(&sid));
    result.set("name", Value::Str(redact::redact_string(name)));
    result.set("kind", Value::Int(1));
    result.set("startTimeUnixNano", Value::text(&begin));
    result.set("endTimeUnixNano", Value::text(&finish));
    result.set("attributes", attributes(attrs)?);
    if let Some(parent) = parent.filter(|parent| !parent.is_empty()) {
        result.set("parentSpanId", Value::text(&parent));
    }
    Ok(Value::Dict(result))
}

fn as_list(value: &Value) -> Result<&[Value]> {
    match value {
        Value::List(items) => Ok(items),
        other => bail!(
            "TypeError: '{}' object is not iterable",
            py::type_name(other)
        ),
    }
}

fn or_empty_dict(value: Value) -> Value {
    if py::truthy(&value) {
        value
    } else {
        Value::Dict(Dict::new())
    }
}

fn task_attr(start: &Value) -> Result<Value> {
    let task = py::get(start, "task", Value::Null)?;
    let task = if py::truthy(&task) {
        task
    } else {
        Value::text("")
    };
    let Value::Str(text) = task else {
        bail!(
            "TypeError: redact_string() argument must be str, not {}",
            py::type_name(&task)
        );
    };
    Ok(Value::Str(redact::redact_string(&text).head(TASK_BUDGET)))
}

fn output_size(result: &Value) -> Result<i128> {
    let output = py::get(result, "output", Value::Null)?;
    let output = if py::truthy(&output) {
        output
    } else {
        Value::text("")
    };
    let Value::Str(text) = output else {
        bail!(
            "AttributeError: '{}' object has no attribute 'encode'",
            py::type_name(&output)
        );
    };
    let Some(utf8) = text.to_utf8() else {
        bail!("UnicodeEncodeError: 'utf-8' codec can't encode a surrogate in tool output");
    };
    Ok(i128::try_from(utf8.len()).expect("output size fits"))
}

/// `convert`: one record to an OTLP `ExportTraceServiceRequest`.
pub fn convert(path: &Path) -> Result<Value> {
    let rows = read_rows(path)?;
    let first = rows.first().expect("read_rows requires a run_start");
    let run_id = py::item(first, "run_id")?;
    let start = py::item(first, "hydra")?;
    let answer = normalized(&rows, run_id)?;
    let projection = or_empty_dict(py::get(&answer, "projection", Value::Null)?);
    let mut end = Value::Dict(Dict::new());
    for row in rows.iter().rev() {
        let hydra = py::get(row, "hydra", Value::Dict(Dict::new()))?;
        let typ = py::get(&hydra, "type", Value::Null)?;
        if py::eq_str(&typ, "run_end") {
            end = py::item(row, "hydra")?.clone();
            break;
        }
    }
    let session = session_id(first, path.parent().unwrap_or(Path::new(".")))?;
    let trace_id = identifier(&concat_str("trace:", &Value::text(&session))?, 32)?;
    let mut times = Vec::new();
    for row in &rows {
        let seq = py::item(row, "seq")?.clone();
        let ts = py::item(row, "ts")?;
        put_time(&mut times, seq, nano(ts)?)?;
    }
    let root_start = nano(py::item(first, "ts")?)?;
    let ended_at = py::get(&end, "ended_at", Value::Null)?;
    let root_end = if py::truthy(&ended_at) {
        nano(&ended_at)?
    } else {
        let last = rows.last().expect("read_rows requires a run_start");
        get_times(&times, py::item(last, "seq")?)?.to_string()
    };
    let usage = or_empty_dict(py::get(&projection, "usage", Value::Null)?);
    let mut values = Dict::new();
    values.set("provider", py::get(first, "provider", Value::Null)?);
    values.set("model", py::get(first, "model", Value::Null)?);
    values.set("policy", py::get(start, "policy", Value::Null)?);
    values.set("outcome", py::get(&projection, "outcome", Value::Null)?);
    values.set("task", task_attr(start)?);
    values.set(
        "usage.source",
        py::get(&usage, "usage_source", Value::Null)?,
    );
    values.set("run.complete", Value::Bool(py::truthy(&end)));
    let counts = or_empty_dict(py::get(&usage, "counts", Value::Null)?);
    if let Value::Dict(counts) = &counts {
        for (key, value) in counts.iter() {
            let mut name = PyStr::from("tokens.");
            name.push(key);
            values.set_key(name, value.clone());
        }
    } else if py::truthy(&counts) {
        bail!(
            "AttributeError: '{}' object has no attribute 'items'",
            py::type_name(&counts)
        );
    }
    let parent = py::get(first, "parent_run_id", Value::Null)?;
    let parent = if py::truthy(&parent) {
        Some(span_id(&parent)?)
    } else {
        None
    };
    let root = span(
        &trace_id,
        &PyStr::from("hydra.run"),
        span_id(run_id)?,
        root_start,
        root_end,
        &values,
        parent,
    )?;
    let root_sid = match &root {
        Value::Dict(dict) => match dict.get("spanId") {
            Some(Value::Str(s)) => s.lossy(),
            _ => String::new(),
        },
        _ => String::new(),
    };
    let mut spans = vec![root];
    let events = or_empty_dict_or_list(py::get(&answer, "events", Value::Null)?);
    let events = as_list(&events)?;
    let mut decisions = Vec::new();
    let mut results = Vec::new();
    for event in events {
        let typ = py::item(event, "type")?;
        if py::eq_str(typ, "permission_decision") {
            put(
                &mut decisions,
                py::item(event, "call_id")?.clone(),
                event.clone(),
            )?;
        } else if py::eq_str(typ, "tool_result") {
            put(
                &mut results,
                py::item(event, "call_id")?.clone(),
                event.clone(),
            )?;
        }
    }
    for event in events {
        if !py::eq_str(py::item(event, "type")?, "tool_call") {
            continue;
        }
        let call_id = py::item(event, "call_id")?;
        let result = get_eq(&results, call_id)
            .cloned()
            .unwrap_or_else(|| Value::Dict(Dict::new()));
        let decision = get_eq(&decisions, call_id)
            .cloned()
            .unwrap_or_else(|| Value::Dict(Dict::new()));
        let begin = get_times(&times, py::item(event, "seq")?)?.to_string();
        let finish = if py::truthy(&result) {
            get_times(&times, py::item(&result, "seq")?)?.to_string()
        } else {
            begin.clone()
        };
        let mut attrs = Dict::new();
        attrs.set("tool.name", py::get(event, "name", Value::Null)?);
        attrs.set(
            "gate.decision",
            py::get(&decision, "decision", Value::Null)?,
        );
        attrs.set("gate.decider", py::get(&decision, "decider", Value::Null)?);
        attrs.set("result.status", py::get(&result, "status", Value::Null)?);
        attrs.set("timing.complete", Value::Bool(py::truthy(&result)));
        if py::truthy(&result) {
            attrs.set("output.size_bytes", Value::Int(output_size(&result)?));
        }
        let mut ident = py::str(run_id);
        ident.push_str(":tool:");
        ident.push(&py::str(py::item(event, "seq")?));
        ident.push_str(":");
        ident.push(&py::str(call_id));
        let name = match py::item(event, "name")? {
            Value::Str(name) => name.clone(),
            other => py::str(other),
        };
        spans.push(span(
            &trace_id,
            &name,
            identifier(&ident, 16)?,
            begin,
            finish,
            &attrs,
            Some(root_sid.clone()),
        )?);
    }
    for row in &rows {
        let body = or_empty_dict(py::get(row, "hydra", Value::Null)?);
        let typ = py::get(&body, "type", Value::Null)?;
        if !py::eq_str(&typ, "seat_request") {
            continue;
        }
        let mut ident = py::str(run_id);
        ident.push_str(":seat:");
        ident.push(&py::str(py::item(row, "seq")?));
        let mut attrs = Dict::new();
        attrs.set("gate.decision", py::get(&body, "decision", Value::Null)?);
        attrs.set("shape", py::get(&body, "shape", Value::Null)?);
        let at = get_times(&times, py::item(row, "seq")?)?.to_string();
        let mut seat = match span(
            &trace_id,
            &PyStr::from("hydra.seat_request"),
            identifier(&ident, 16)?,
            at.clone(),
            at,
            &attrs,
            Some(root_sid.clone()),
        )? {
            Value::Dict(dict) => dict,
            other => bail!("TypeError: span is {}", py::type_name(&other)),
        };
        let runs = py::get(&body, "runs", Value::List(Vec::new()))?;
        let mut links = Vec::new();
        for child in as_list(&runs)? {
            let mut link = Dict::new();
            link.set("traceId", Value::text(&trace_id));
            link.set("spanId", Value::text(&span_id(child)?));
            links.push(Value::Dict(link));
        }
        seat.set("links", Value::List(links));
        spans.push(Value::Dict(seat));
    }
    let mut resource_attrs = Dict::new();
    resource_attrs.set("service.name", Value::text("hydra"));
    let mut resource = Dict::new();
    resource.set("attributes", attributes(&resource_attrs)?);
    let mut scope = Dict::new();
    scope.set("name", Value::text("hydra.otel"));
    let mut scope_span = Dict::new();
    scope_span.set("scope", Value::Dict(scope));
    scope_span.set("spans", Value::List(spans));
    let mut resource_span = Dict::new();
    resource_span.set("resource", Value::Dict(resource));
    resource_span.set("scopeSpans", Value::List(vec![Value::Dict(scope_span)]));
    let mut out = Dict::new();
    out.set(
        "resourceSpans",
        Value::List(vec![Value::Dict(resource_span)]),
    );
    Ok(Value::Dict(out))
}

fn or_empty_dict_or_list(value: Value) -> Value {
    if py::truthy(&value) {
        value
    } else {
        Value::List(Vec::new())
    }
}

/// `write_json` of the Python, reused from `hydra-records`.
pub fn write_json(path: &Path, value: &Value) -> Result<()> {
    records::write_json(path, value)
}

/// `json.dump` to stdout with `indent=2` and a trailing newline.
pub fn dump_stdout(stdout: &mut dyn Write, value: &Value) -> Result<()> {
    records::refuse_nan(value)?;
    let text = json::dumps(value, Some(2)) + "\n";
    write!(stdout, "{text}").map_err(|err| anyhow!("OSError: {err}"))?;
    Ok(())
}

fn glob_jsonl(name: &std::ffi::OsStr) -> bool {
    name.to_str().is_some_and(|name| name.ends_with(".jsonl"))
}

fn walk_jsonl(dir: &Path, found: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).map_err(|err| io_err(&err, dir))?;
    let mut entries: Vec<_> = entries.collect();
    for entry in entries.drain(..) {
        let entry = entry.map_err(|err| io_err(&err, dir))?;
        let path = entry.path();
        let ft = entry.file_type().map_err(|err| io_err(&err, &path))?;
        let matches = glob_jsonl(path.file_name().unwrap_or_default());
        if ft.is_symlink() {
            if matches {
                found.push(path);
            }
            continue;
        }
        if ft.is_dir() {
            walk_jsonl(&path, found)?;
            if matches {
                found.push(path);
            }
        } else if matches {
            found.push(path);
        }
    }
    Ok(())
}

fn rglob_jsonl(root: &Path) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    walk_jsonl(root, &mut found)?;
    found.sort_by(|a, b| {
        a.components()
            .map(|c| c.as_os_str())
            .cmp(b.components().map(|c| c.as_os_str()))
    });
    Ok(found)
}

fn stamp_matches(stamp: &Path, size: u64) -> Result<bool> {
    if !stamp.is_file() {
        return Ok(false);
    }
    let text = read_text(stamp)?;
    let loaded = match json::loads_explained(&PyStr::from(text)) {
        Ok(value) => value,
        Err((error, message)) => return Err(json_err(error, message)),
    };
    let size = i128::from(size);
    Ok(py::eq(&loaded, &Value::Int(size)))
}

/// `batch`: every `.jsonl` under `runs`, skipped when the size stamp still matches.
pub fn batch(runs: &Path, out: &Path) -> Result<()> {
    for path in rglob_jsonl(runs)? {
        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let target = out.join(format!("{stem}.json"));
        let stamp = out.join(format!("{stem}.size.json"));
        let size = fs::metadata(&path)
            .map_err(|err| io_err(&err, &path))?
            .len();
        if target.is_file() && stamp_matches(&stamp, size)? {
            continue;
        }
        write_json(&target, &convert(&path)?)?;
        write_json(&stamp, &Value::Int(i128::from(size)))?;
    }
    Ok(())
}

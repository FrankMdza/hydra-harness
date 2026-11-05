# 140 notes: what the two videos say, read 2026-11-04

The session read both in Frank's Chrome, since WebFetch got only the title of each page. These are
notes, not the transcripts.

## The short, `jNuDdqEwI58`

Read: the title, "GPT-6 Just Fixed Another AI by Editing Its Weights!", tags #ai #openai #llm. The
Chrome extension disconnected before the session could open its transcript. This matches the
thumbnail reading in `138-nano-from-scratch.md` section 1 ("AI JUST FIXED ANOTHER AI" over a weight
matrix). Research 140 opens the short and its primary source.

## The video, `os5eNeAVRpc`

"Amazon built a new AI agent runtime in Rust", Let's Get Rusty, published 2026-11-02, 7:06. Source
it cites: <https://aws.amazon.com/blogs/opensource/introducing-infinity-a-runtime-for-extreme-agent-concurrency/>.
Read from its auto-generated English transcript and chapters:

- AWS released Infinity, a Rust framework for very concurrent agent systems. Its homepage, as the
  video quotes it, says 75,000 agents fit in a Raspberry Pi's memory, about 8 GB.
- The problem it names: agents spend most of their life waiting on builds, webhooks and humans, and
  a runtime that holds one process per agent pays for that wait.
- Infinity runs an agent in slices. A slice starts when one or more messages arrive for a thread
  (a tool result, user text or an event). The runtime loads the message history from durable
  storage, preprocesses, calls the model provider (Anthropic and OpenAI given as examples),
  persists the slice's state, and if the model asked for a tool, POSTs it to a tool server. Then
  the agent sleeps.
- The tool server speaks RAP, the Reactive Agent Protocol. It runs the tool while the runtime does
  other work, and when the tool finishes (minutes, hours or days later) it sends a message back to
  the input queue, which starts a new slice. A tool call can stay open as a stream of events.
- The video maps this onto Rust's async/await: a slice is a future, a tool call is an `.await`
  point, and the saved state is the state machine.
- Everything is a message. Events come back as synthetic tool calls, named with an `_injected`
  suffix, rather than as user messages, so the model can tell an event from the human. The example
  is a `spawn_thread` call whose child, when done, answers with an injected tool call.
- Because no state lives in the process between slices, the runtime can shut down completely and
  scale to zero, which is why it suits AWS Lambda.

Not in the video: the repository, licence, version, which providers it supports beyond the two
examples, how it handles a provider CLI rather than a raw API, and any benchmark beyond the 75,000
figure.

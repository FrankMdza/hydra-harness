# 312 - The builder report over the build tickets: who built what, and whether it stood

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-13, by the session. Build 301 printed acceptance and cost per task class and
model from the goal logs, and found two goals and eleven tasks. The last month's builds did not
go through goal logs: they went through `hydra run` from a ticket under `.scratch/hydra/build/`,
and each Outcome names the run, the provider and model, the tokens, and what the session did
with the diff. Sixty-three tickets name a run id today. That corpus is what the human's
question about a model that learns to delegate needs, and nothing prints it.

## What

An Outcome reads like "Built by `codex` `gpt-6.1-sol` on Hydra run `01a1340b-...` under
`standard`, 3.13M tokens, applied to master on 2026-10-13 after the diff was read whole", or
"Built 2026-09-24 by `codex` on run 01a0d36f-4447 (1.56M tokens)", or "Built by `composer-2.5`
on run 01a0d587-e3e1 under `standard`, about 0.59M". The phrasing varies, the run id is the
constant: a uuid-v7 prefix of at least 13 characters starting `01a`. The run record carries the
truth the prose repeats: provider, model, policy, outcome, tokens, duration. The prose carries
what the record cannot: whether the session applied the diff as it was, corrected it, finished
it by hand, or rejected it.

## Do

1. `hydra-puppets report tickets --dir .scratch/hydra/build --runs <runs dir>`: for every
   ticket file with `Type: build`, take the text under `## Outcome`, find every run id prefix in
   it (`01a` plus hex and hyphens, 13 characters or more), resolve each against the run records
   (the record file name starts with the id; a prefix that matches two is ambiguous and is
   listed as such), and read provider, model, policy, outcome and total tokens from the record
   with the readers `hydra-puppets` already has for `run_end` and usage (read `wait.rs` and
   `report.rs` first; do not parse the record by hand if a reader exists).
2. Classify the prose with a small fixed vocabulary, and print the classification beside the
   phrase it matched so a reader can disagree: `applied as is` (phrases such as "applied as it
   was", "entered without correction", "aplicado tal cual"), `corrected` ("needed correction",
   "the session fixed", "with one fix", "amended by the session"), `finished by hand` ("the
   session finished", "died", "capacity", "completed the mechanical rest"), `rejected`
   ("rejected", "not applied", "discarded"), and `unsaid` when none matched. Keep the phrase
   list in one place at the top of the module with a comment saying it is a heuristic over
   prose and will be wrong sometimes.
3. Print two tables. Per provider and model: tickets, runs, tokens total and median, outcome
   counts from the records (`completed`, `failed`, the rest), and the four prose classes.
   Per ticket, sorted by number: ticket, runs, model, tokens, record outcome, prose class and
   the matched phrase. Run it on this repo and put both tables in the Outcome, whole.
4. One test under `crates/puppets/tests/` on a fixture directory of four tickets and three
   fake records, covering: a ticket with two runs, an ambiguous prefix, a run id with no
   record, and each prose class once.
5. `integrations/puppets/README.md` gains the command beside `report builders`. Amend
   `IMPLEMENTATION.md` where `report.rs` is described.

## Done means

The two tables in the Outcome, with the `unsaid` count stated plainly so the human knows how
much the prose fails to say; the test; `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo nextest run --release -p hydra-puppets` green.

## Outcome

Implemented `report tickets` in `crates/puppets/src/report/tickets.rs`, alongside the
existing goal-log report. Run references are resolved by filename, with ambiguous and
missing prefixes left visible; normalization reuses `launch.rs`'s end check and show
reader. Envelope provider/model reflect model backfill, policy comes from the start,
and outcome/usage come from the normalized projection. Runs and per-builder tickets
are deduplicated; unknown token totals remain unavailable. The fixed phrase list is
heuristic, and each ticket prints its matched phrase. Repeated Outcome headings in
existing tickets are included rather than hiding the second heading's content.

The four-ticket, three-fake-record command fixture covers two runs in one ticket,
ambiguity, missing records, each prose class, ordering, duplicate mentions, model
backfill, odd/even medians, unsaid and the tickets-only view. No dependency was added.
README and IMPLEMENTATION document the command and counting rules. The requested
`crates/puppets/src/wait.rs` does not exist in this tree; `wait` and its record readers
are in `crates/puppets/src/launch.rs`, which was read instead.

Validation passed: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, and
`cargo nextest run --release -p hydra-puppets` (98 passed, 1 skipped).

Real run records are inaccessible under this run's policy. No records outside the
worktree were read or copied. The following two tables were printed by the built
binary with `report tickets --dir .scratch/hydra/build`, before appending this Outcome.
Record-derived columns are unavailable, and the aggregate row has no provider/model
attribution; its prose counts come from tickets alone. There are **286 unsaid out of
313 build tickets**. The session must run the binary with `--runs <records directory>`
against its authorized record copy to obtain the real provider/model totals and record
columns. This snapshot precedes the report's inclusion in this ticket's own Outcome;
a later report will also scan the run references and phrases printed here.

Run by the session on master against the real records (`--runs ~/hydra/runs`, 913 records,
2026-10-13). The totals table is whole; the per-ticket table follows it, whole, as the ticket
asked. Read the prose columns with the heuristic in mind: 285 of 313 tickets are `unsaid`, and
`applied as is` is 0 everywhere because no Outcome uses the three phrases in the list; the
session's own Outcomes say "applied to master" or "entered without correction" in other words.
`rejected` fires on the word itself, which build 303's Outcome uses for a verdict class and build
310's for a discarded measurement, so those two rows are wrong. The record columns are the
reliable half: tokens, outcome and model per run.

```tsv
provider	model	tickets	runs	total_tokens	median_tokens	completed	failed	rest	applied as is	corrected	finished by hand	rejected
agy	gemini-3.1-pro-high	7	4	7446149	1662292.5	3	1	0	0	0	1	1
agy	gemini-3.8-flash-high	4	3	6943174	3220223	2	1	0	0	0	1	1
agy	gemini-3.8-flash-low	5	4	83573	20591.5	4	0	0	0	0	0	1
agy	unavailable	3	2	5848799	2924399.5	2	0	0	0	0	0	1
claude	claude-haiku-4-5-20251001	17	15	15029637	50581	15	0	0	0	0	1	2
claude	claude-opus-5-5	52	55	278053495	3080943	55	0	0	0	0	1	3
claude	claude-opus-5[1m]	5	4	559181	34870.5	3	0	1	0	0	1	1
claude	claude-sonnet-5	11	9	112142475	7354136	9	0	0	0	0	0	1
claude	claude-sonnet-5-5	13	12	11740626	748124	12	0	0	0	0	1	1
claude	haiku	3	2	165364	82682	0	0	2	0	0	0	1
claude	sonnet	4	2	unavailable	unavailable	1	1	0	0	1	0	1
claude	unavailable	3	2	unavailable	unavailable	1	0	1	0	0	0	1
codex	gpt-6-astra	2	1	381697	381697	1	0	0	0	0	0	1
codex	gpt-6.1-sol	25	26	unavailable	unavailable	25	1	0	0	0	2	2
codex	unavailable	22	33	unavailable	unavailable	21	6	6	0	0	3	2
cursor-agent	Composer 2.5	78	81	191716503	1350313	81	0	0	0	1	5	3
cursor-agent	Cursor Grok 4.6 High Fast	7	6	unavailable	unavailable	5	0	1	0	0	0	1
cursor-agent	Grok 4.6 High	17	16	68037444	4094903	16	0	0	0	1	0	1
cursor-agent	Grok 4.7 256K High	9	8	20756025	2589715	8	0	0	0	0	0	1
cursor-agent	composer-2.5	2	1	unavailable	unavailable	0	1	0	0	0	1	1
cursor-agent	gpt-5.6-sol-high	2	1	unavailable	unavailable	0	1	0	0	0	1	1
unavailable	unavailable	5	8	unavailable	unavailable	7	1	0	0	0	1	1

ticket	runs	model	tokens	record outcome	prose class	matched phrase	policy
1	-	unavailable	unavailable	unavailable	unsaid		unavailable
2	-	unavailable	unavailable	unavailable	unsaid		unavailable
3	-	unavailable	unavailable	unavailable	unsaid		unavailable
4	01a07308-a250-7792-9e8b-0d99697c99d9; 01a07218-8817-7581-9ddb-47565e5d5ba7; 01a073ef-701c-7522-b5bb-56d5ecba0f57	claude-opus-5[1m]; claude-sonnet-5; claude-opus-5[1m]	86591	completed; completed; rate_limited	unsaid		standard; unavailable; read-only
5	01a07471-0a3e-74d2-b576-153ac0e7427b; 01a073ef-701c-7522-b5bb-56d5ecba0f57	claude-opus-5[1m]; claude-opus-5[1m]	24073	completed; rate_limited	unsaid		read-only; read-only
6	-	unavailable	unavailable	unavailable	finished by hand	died	unavailable
7	-	unavailable	unavailable	unavailable	unsaid		unavailable
8	-	unavailable	unavailable	unavailable	unsaid		unavailable
9	01a07891-a339-7173-96d9-70beb88e5efb; 01a07892-459d-7420-904e-91d6a6986159	Composer 2.5; Composer 2.5	173254	completed; completed	unsaid		standard; read-only
10	01a07915-c9cf-7cc0-b156-204771091a76	gemini-3.8-flash-low	29486	completed	unsaid		read-only
11	-	unavailable	unavailable	unavailable	unsaid		unavailable
12	01a07513-baa3-7cf0-b20a-ded11ede6a38	claude-opus-5[1m]	489440	completed	finished by hand	died	curate
13	01a078bc-420f-7b23-8e67-2dfc9f98f859; 01a07512-d36e-7651-b8d9-e9e279e5730b	claude-haiku-4-5-20251001; claude-haiku-4-5-20251001	128551	completed; completed	rejected	rejected	standard; standard
14	01a07513-baa3-7cf0-b20a-ded11ede6a38	claude-opus-5[1m]	489440	completed	unsaid		curate
15	-	unavailable	unavailable	unavailable	unsaid		unavailable
16	-	unavailable	unavailable	unavailable	unsaid		unavailable
17	01a07eed-700a-7020-b503-1f5276f0cb15; 01a07eef-329c-7501-a201-d959a53c8777; 01a07eed-9c4a-7633-887d-e3c143af6dae	Composer 2.5; claude-haiku-4-5-20251001; unavailable	2857822	completed; completed; completed	unsaid		standard; read-only; standard
18	01a07efe-97f4-7ae1-8ab7-308dfa14f439	claude-sonnet-5	7354136	completed	unsaid		standard
19	01a07efb-de49-7a70-81e4-76fa23c79d52	gemini-3.1-pro-high	1078004	failed	finished by hand	died	standard
20	01a07f02-36a9-7f21-95ed-5ddf27b5f317	Composer 2.5	627799	completed	unsaid		standard
21	01a07f05-7a50-78b1-aaf1-48eaee7a75a4	Cursor Grok 4.6 High Fast	972778	completed	unsaid		standard
22	01a07f0d-2d90-7241-bef1-e96ef90a1012; 01a07efe-97f4-7ae1-8ab7-308dfa14f439	claude-sonnet-5; claude-sonnet-5	19247557	completed; completed	unsaid		standard; standard
23	01a07f18-910f-7282-8bda-44347806e3db; 01a07f27-1683-72c3-946c-47885dd3f679	claude-sonnet-5; claude-haiku-4-5-20251001	10647666	completed; completed	unsaid		standard; read-only
24	01a07f18-9a28-7111-971c-f7e260c13cdc	Composer 2.5	581683	completed	unsaid		standard
25	01a07f37-8a67-7542-bdd9-e4c2a630cfe7	claude-haiku-4-5-20251001	4743835	completed	unsaid		standard
26	01a07f37-9461-7200-ad1a-4d4e50f9e395	claude-sonnet-5	2647719	completed	unsaid		standard
27	01a07f37-9cab-71c2-b114-73bb18b2b23a	claude-sonnet-5	54681815	completed	unsaid		standard
28	01a07f62-7061-7ea2-83ce-1c2e47c95c1d; 01a07ff5-2389-75f1-a3eb-7296adde0a9b	Composer 2.5; haiku	5816721	completed; gated	unsaid		standard; read-only
29	01a07ffe-372a-7731-bab0-d46c03eaf0d9; 01a07f6c-ba21-7622-b5e1-813a9d9c60db	Composer 2.5; haiku	1152962	completed; rate_limited	unsaid		standard; read-only
30	01a07ffe-4142-7c40-8440-de8bbe6370f6	Cursor Grok 4.6 High Fast	1924447	completed	unsaid		standard
31	01a07ffe-4808-7690-9b25-a72b854da243; 01a08008-ba33-7ef1-a142-82ad907cbf55	gemini-3.8-flash-high; Composer 2.5	4433371	failed; completed	finished by hand	died	standard; standard
32	01a0801d-7382 (missing); 01a08042-7cf1 (missing); 01a07f3f-3241-7231-9066-78838d46a73d	unavailable; unavailable; claude-haiku-4-5-20251001	unavailable	missing; missing; completed	unsaid		unavailable; unavailable; standard
33	01a0801d-7832 (missing); 01a07efb-de49-7a70-81e4-76fa23c79d52	unavailable; gemini-3.1-pro-high	unavailable	missing; failed	unsaid		unavailable; standard
34	01a0801d-7811 (missing)	unavailable	unavailable	missing	unsaid		unavailable
35	01a08052-ccdc-7cc1-b28a-9c35de0bc986; 01a0800f-1fa4-7e92-8637-f6a4220d5d4e; 01a07efb-de49-7a70-81e4-76fa23c79d52	Cursor Grok 4.6 High Fast; Composer 2.5; gemini-3.1-pro-high	8830345	completed; completed; failed	unsaid		standard; read-only; standard
36	01a08053-3d85-7f40-97c9-059aed4da16c; 01a08127-6f58-7f22-9798-934a8cb414e5; 01a08129-de0c-73f0-aa74-927b6efd0dd9; 01a0812a-ad31-7810-ba8e-97ce189115f2; 01a0814a-c730-7e11-847b-fac691b5d57b; 01a08169-eb3b-7053-9d0c-797a62ad1a37; 01a0822e-bc48-7e31-b36e-c2d266dc9c44; 01a08241-831d-74a3-8ce5-122dd0c70a6f	Composer 2.5; unavailable; unavailable; claude-haiku-4-5-20251001; unavailable; unavailable; unavailable; unavailable	unavailable	completed; failed; completed; completed; completed; completed; completed; completed	finished by hand	died	standard; unavailable; unavailable; standard; unavailable; unavailable; unavailable; unavailable
37	01a08067-548b-7223-8d7c-a2709100486e; 01a07f37-9cab-71c2-b114-73bb18b2b23a; 01a08111-1e5e-7051-a541-971996a7fc27	Cursor Grok 4.6 High Fast; claude-sonnet-5; Composer 2.5	57362326	completed; completed; completed	unsaid		standard; standard; standard
38	01a08112-fe30-7560-bb0b-88a14f3d2e1c; 01a0811e-d401-7452-a242-52329b8150d1; 01a08111-1e5e-7051-a541-971996a7fc27	Cursor Grok 4.6 High Fast; Composer 2.5; Composer 2.5	3118789	completed; completed; completed	unsaid		standard; standard; standard
39	01a084d3-bdd3-7831-af07-339c253faf33	unavailable	unavailable	completed	unsaid		unavailable
40	-	unavailable	unavailable	unavailable	unsaid		unavailable
41	-	unavailable	unavailable	unavailable	rejected	rejected	unavailable
42	-	unavailable	unavailable	unavailable	unsaid		unavailable
43	-	unavailable	unavailable	unavailable	unsaid		unavailable
44	01a09405-f451 (ambiguous)	unavailable	unavailable	ambiguous	unsaid		unavailable
45	01a0902c-8d66-7e10-80a6-2d36c7879118	unavailable	3178822	completed	unsaid		standard
46	01a07f37-8a67-7542-bdd9-e4c2a630cfe7; 01a095e6-626f-7bf2-ab17-7074eb9d4674	claude-haiku-4-5-20251001; claude-sonnet-5	10753668	completed; completed	unsaid		standard; standard
47	-	unavailable	unavailable	unavailable	unsaid		unavailable
48	01a0bcdb-e6ed-7180-8a96-3674b960cbc8; 01a0c220-909f-7432-b1a7-8292767278e7; 01a0c220-ce0e-7433-b869-343b7b6e6c6b; 01a0c223-9ec2-72e0-b5d4-e29782d692b0; 01a0c222-4305-74f0-a378-2b6c57e03ea4; 01a0c231-6583-7c72-b10e-981a2cc0d6fb	Composer 2.5; unavailable; unavailable; unavailable; unavailable; unavailable	unavailable	completed; failed; failed; completed; failed; completed	finished by hand	died	standard; standard; standard; standard; standard; standard
49	-	unavailable	unavailable	unavailable	unsaid		unavailable
50	01a0a59c-c395-7423-87e5-3dadfb52a4e1	Composer 2.5	1296792	completed	unsaid		standard
51	01a0a56e-c71f-7783-bf25-2a6a16ece5ef; 01a0a56f-9a20-7c93-a618-d712c9a618db	Cursor Grok 4.6 High Fast; Composer 2.5	unavailable	crashed; completed	unsaid		standard; standard
52	-	unavailable	unavailable	unavailable	unsaid		unavailable
53	01a07f3f-3241-7231-9066-78838d46a73d; 01a07514-ac48-7bd2-8838-ea6a16fd3e9d	claude-haiku-4-5-20251001; unavailable	unavailable	completed; skill_collision	unsaid		standard; standard
54	-	unavailable	unavailable	unavailable	unsaid		unavailable
55	-	unavailable	unavailable	unavailable	unsaid		unavailable
56	-	unavailable	unavailable	unavailable	unsaid		unavailable
57	-	unavailable	unavailable	unavailable	unsaid		unavailable
58	-	unavailable	unavailable	unavailable	unsaid		unavailable
59	-	unavailable	unavailable	unavailable	unsaid		unavailable
60	01a0a572-e4ab-7e70-bf9d-cbad277b6040	Composer 2.5	1233966	completed	unsaid		standard
61	01a0c4a2-d6b4-71a3-a2e5-f1eba83d74fa; 01a0c4b4-81ec-7752-ab39-451ff5dfc144; 01a0c4c9-9ad1-79a1-9734-8bbe0fc5dd6a	Composer 2.5; unavailable; unavailable	unavailable	completed; completed; completed	unsaid		standard; unavailable; judge
62	01a0c4a6-cf6d-7620-99ff-9d28de8f0c73	Composer 2.5	2453505	completed	unsaid		standard
63	01a0c5dd-5e0a-7881-8a3c-bb30b7edbb6c; 01a0c61b-9816-71e3-83d1-f155aa710b47	unavailable; Composer 2.5	1528069	completed; completed	unsaid		standard; standard
64	01a0c624-12c5-7653-a890-d8ba05bcdb83	Composer 2.5	2075684	completed	unsaid		standard
65	01a0c5dd-984e-7730-8d36-b3f4e3334763	Composer 2.5	5110586	completed	unsaid		standard
66	01a0c5dd-bf97-7793-870f-395526f53a88	Composer 2.5	469422	completed	unsaid		standard
67	01a0c61d-0639-7580-aad9-4f471d6ae619	Composer 2.5	1893342	completed	unsaid		standard
68	01a0c61d-4289-7c01-9676-1d51f718c3ce; 01a0c62d-b98d-75e3-bf26-b789306aa17d; 01a0c6a5-be8c-7ea3-aa56-0772f8d6e819; 01a0c6a7-9315-7132-b87c-28804b7c8075	Composer 2.5; unavailable; composer-2.5; unavailable	unavailable	completed; completed; failed; completed	finished by hand	died	standard; standard; standard; standard
69	01a0c61e-e2b6-70d1-9198-de7a1497d42a	Composer 2.5	2173794	completed	unsaid		standard
70	01a0c924-5609-70e3-97a7-546cdeb981c4	Composer 2.5	5390612	completed	unsaid		standard
71	01a0c924-7d46-7993-a0aa-fb8d64eec576	Composer 2.5	746423	completed	unsaid		standard
72	01a0c924-a494-7e82-bad2-5384686a42dd	Composer 2.5	11837366	completed	unsaid		standard
73	01a0c9ca-1591-7901-8d8f-1526e7e50287; 01a0c955-f7a1-70d0-988b-c5b0780d04b2	unavailable; Composer 2.5	937695	completed; completed	unsaid		standard; standard
74	01a0c960-a3a5-7dc3-a450-939485eb869c; 01a0c960-c1f5-7b10-9016-6b6d29546ff5; 01a0c960-e174-72f3-8a7f-7dfc4dd20a10; 01a0c961-009d-7753-9134-25324c1115c4; 01a0c956-213e-7f80-908a-2c6c6e29c436	Composer 2.5; unavailable; gemini-3.8-flash-high; claude-haiku-4-5-20251001; Composer 2.5	1748874	completed; completed; completed; completed; completed	unsaid		standard; standard; standard; standard; standard
75	01a0c956-462b-7542-924f-9bdf9d6c72f6	Composer 2.5	2793247	completed	unsaid		standard
76	-	unavailable	unavailable	unavailable	unsaid		unavailable
77	01a0cc2a-d2a1-7b32-93ec-0d115885da73; 01a0cc4a-ad67-7c22-a5a1-9b2241d4f88e	Composer 2.5; unavailable	2446739	completed; completed	unsaid		standard; standard
78	01a0cc32-1103-77e1-9db7-4317d5d07e0b	Composer 2.5	1157022	completed	unsaid		standard
79	01a0cc3f-1e0f-7b20-87c0-dc55f4c0ce62	Composer 2.5	1969135	completed	unsaid		standard
80	-	unavailable	unavailable	unavailable	unsaid		unavailable
81	01a0cc2b-e29c-72a1-85a6-d13cc27eece8; 01a0cc2a-f20b-7d91-abb6-41f991c7be79	unavailable; unavailable	761479	completed; completed	unsaid		standard; standard
82	01a0cc2b-10d6-7311-b935-de2988618d7a	claude-haiku-4-5-20251001	4017617	completed	unsaid		standard
83	01a0cc3f-4438-7913-b52a-88c94518a149	unavailable	714961	completed	unsaid		standard
84	-	unavailable	unavailable	unavailable	unsaid		unavailable
85	01a0cc32-3831-7d42-b307-0248751bb76c	unavailable	617842	completed	unsaid		standard
86	01a0c924-a494-7e82-bad2-5384686a42dd; 01a0cc32-5f53-7ee0-822c-fa6e8d5293fb	Composer 2.5; Composer 2.5	15082720	completed; completed	unsaid		standard; standard
87	01a0cc69-bbd2-7453-b4b6-cd2d8df859fe; 01a0cc9b-5397-7123-9e00-1ce4a51db97b	Composer 2.5; unavailable	unavailable	completed; skill_collision	rejected	discarded	standard; standard
88	01a0ccb1-2fea-7e13-a089-3930a184b5c8; 01a0ccb1-681d-7ab0-9e67-1dcb763c638e; 01a0ccb1-83c0-7761-a1d1-28d9a8b2c7cb	claude-haiku-4-5-20251001; Composer 2.5; gemini-3.8-flash-low	65420	completed; completed; completed	unsaid		read-only; read-only; read-only
89	01a0cf91-1d0b-7d02-b4fa-d2947cce86e7; 01a0cf8d-ce29-7cf0-9d96-5d924af9ce69	unavailable; unavailable	966517	completed; completed	unsaid		standard; standard
90	01a0cf91-7525-7fa3-9ff1-bd67c9d66f29	Composer 2.5	1546320	completed	unsaid		standard
91	01a0cfa1-2185-7000-9ef6-4ced5d767e0f	claude-opus-5-5	1549879	completed	unsaid		standard
92	01a0cfd5-cdda-7182-915a-4a266342c74d; 01a0cfda-7012-7d43-93ab-224902b0701e; 01a0cfdb-e5c3-72e0-b965-185dcf5b6f8b; 01a0cfdc-ac67-7e63-a807-2333c7c271c6; 01a0cfec-faa6-7ad2-a7d3-7204f59b05d8; 01a0cff3-6576-7af0-b60b-7b59b18614be; 01a0cff3-8452-7f11-af24-aebd47904d41; 01a0cff3-a299-7161-b3d4-3a7566642a8e	unavailable; unavailable; unavailable; Composer 2.5; Composer 2.5; claude-haiku-4-5-20251001; Composer 2.5; gemini-3.8-flash-low	unavailable	completed; completed; rate_limited; completed; completed; completed; completed; completed	unsaid		standard; standard; standard; standard; standard; read-only; read-only; read-only
93	01a0cfa1-507d-7a61-99b1-18b7f5a30b0b	Composer 2.5	3088929	completed	unsaid		standard
94	01a0cf8d-1e06-7892-be75-aba714e94a7d	gemini-3.1-pro-high	3134983	completed	unsaid		standard
95	01a0cfae-6b3a-7d62-a28e-09bee301c51f; 01a0cfa7-7c3e-7c50-8d2c-03c78e8c2f46; 01a0cc56-c15b-7ad2-9741-1dcce176e0a9; 01a0cf89-8b3d-77c1-9f7e-1b11c2a9a41b; 01a0cfcb-4fd1-77c2-b310-d1968148bd77	unavailable; unavailable; claude-sonnet-5; claude-opus-5-5; claude-haiku-4-5-20251001	21827399	completed; completed; completed; completed; completed	unsaid		standard; standard; standard; standard; standard
96	01a0cfa7-d9c9-7f20-a8d1-b65328240674	claude-opus-5-5	3889610	completed	unsaid		standard
97	01a0d1fa-c301-7893-a2a5-019e63033a67	claude-haiku-4-5-20251001	57711	completed	unsaid		standard
98	-	unavailable	unavailable	unavailable	rejected	discarded	unavailable
99	-	unavailable	unavailable	unavailable	unsaid		unavailable
100	01a0cfdb-e5c3-72e0-b965-185dcf5b6f8b; 01a0d02d-f812-7880-bd18-7ec35b213e0a; 01a0cc9d-8f15-7a00-a973-c2628fd9fa30; 01a0ca7d-f703-73b3-8598-a89d742d6ebf	unavailable; unavailable; unavailable; unavailable	unavailable	rate_limited; rate_limited; rate_limited; failed	unsaid		standard; standard; standard; standard
101	-	unavailable	unavailable	unavailable	unsaid		unavailable
102	01a0c62b-d005-7180-8a65-6dc776facdac; 01a0cc9d-8f15-7a00-a973-c2628fd9fa30	unavailable; unavailable	unavailable	gated; rate_limited	unsaid		standard; standard
103	01a0d227-23ce-7833-85df-11b064b8d764; 01a0d226-d533-7ed3-ab50-f351c74a61ef; 01a0d227-ef37-7a22-b465-f53f807d0c09; 01a0d228-206c-7400-b190-07da21ebd542	claude-haiku-4-5-20251001; unavailable; Composer 2.5; gemini-3.8-flash-low	162945	completed; completed; completed; completed	unsaid		standard; standard; standard; standard
104	-	unavailable	unavailable	unavailable	unsaid		unavailable
105	-	unavailable	unavailable	unavailable	unsaid		unavailable
106	01a0cc9d-8f15-7a00-a973-c2628fd9fa30	unavailable	unavailable	rate_limited	unsaid		standard
107	-	unavailable	unavailable	unavailable	unsaid		unavailable
108	01a0d36f-1612-73e1-81ae-41805679498f	Composer 2.5	1345891	completed	unsaid		standard
109	01a0d36f-4447-7b22-b59b-af98eb122a01	unavailable	814402	completed	unsaid		standard
110	01a0d41b-c2cf-7a90-ac05-d94a965a1ba8	Composer 2.5	1260183	completed	unsaid		standard
111	01a0d41e-5b15-7420-8a07-dd925979f284; 01a0d41b-f171-7991-a9cb-0513a0fa9c79	Composer 2.5; unavailable	unavailable	completed; rate_limited	finished by hand	died	standard; standard
112	-	unavailable	unavailable	unavailable	unsaid		unavailable
113	01a0d3e2-1098-78a2-bf8f-c5f8319a8311; 01a0d3e4-4245-70d2-807f-78986913a670	Composer 2.5; claude-opus-5-5	17044314	completed; completed	unsaid		standard; standard
114	01a0d3f5-50e5-7360-89fa-beef48855d14	Composer 2.5	5608330	completed	unsaid		standard
115	01a0d3e4-cd16-74e3-aa61-c77c8c24f3cd	gemini-3.8-flash-high	3709011	completed	unsaid		standard
116	01a0d3ec-aba2-7aa3-8c0f-fe0d28d898db	Composer 2.5	2212456	completed	unsaid		standard
117	01a0d3ec-aba2-7352-940b-e3789394347c	Composer 2.5	1674918	completed	unsaid		standard
118	01a0d428-9ddf-7af2-85d5-cc948fa61616	Composer 2.5	1352842	completed	unsaid		standard
119	01a0d3f8-5272-7f20-95af-8e5e257e435c	Composer 2.5	856877	completed	unsaid		standard
120	01a0d3f5-50e5-73b1-8726-55882f5caf91	Composer 2.5	693171	completed	unsaid		standard
121	01a0d41e-8a6c-7be1-9fcb-42abb59e80fb	Composer 2.5	864884	completed	unsaid		standard
122	01a0d425-68fa-7ad1-9eb3-af1fe207126b	Composer 2.5	1289451	completed	unsaid		standard
123	01a0d56c-a2b7-7552-9c8f-dcf65f4a3c71	Composer 2.5	3031397	completed	unsaid		standard
124	01a0d572-9dee-7743-8038-ad6ec119fbb9	Composer 2.5	5355174	completed	unsaid		standard
125	01a0d56c-a2b7-7491-b595-613283a958a1	Composer 2.5	1018041	completed	unsaid		standard
126	01a0d57a-ab27-74c2-bbdf-f620f1ca1597; 01a0d484-2d8c-7443-b46e-0e4f151c47e0; 01a0d56e-ba1b-7c72-8a44-0dc29389cae9	Composer 2.5; unavailable; sonnet	unavailable	completed; failed; failed	unsaid		standard; standard; standard
127	01a0d587-e3e1-7523-9a7a-cfca460be192	Composer 2.5	593464	completed	unsaid		standard
128	01a0d617-f5a3-7f61-9e76-8ddc4bb3101f	Composer 2.5	1540854	completed	unsaid		standard
129	01a0d618-3015-7f53-be11-90ab34c5eab3	Composer 2.5	608639	completed	unsaid		standard
130	-	unavailable	unavailable	unavailable	unsaid		unavailable
131	01a0d618-6b6e-7b73-9bd2-760f341247eb; 01a0d56e-ba1b-7c72-8a44-0dc29389cae9	Composer 2.5; sonnet	unavailable	completed; failed	corrected	with one fix	standard; standard
132	01a0d618-a6fd-7523-8a20-cefb63b79214	Composer 2.5	959037	completed	unsaid		standard
133	01a0ded9-8e70-7ec2-87d3-564eb17c62db	Composer 2.5	1810342	completed	unsaid		standard
134	01a0ded9-c9a6-70f0-8d05-c8fd34921f35	Composer 2.5	902320	completed	unsaid		standard
135	01a0e70c-f8cc-78f0-b91a-e8358ac208c1	Composer 2.5	604964	completed	unsaid		standard
136	01a0e70c-f8e8-7bf1-9168-ef5bc83335fa	claude-sonnet-5	4710412	completed	unsaid		standard
137	-	unavailable	unavailable	unavailable	finished by hand	The session finished	unavailable
138	-	unavailable	unavailable	unavailable	corrected	The session fixed	unavailable
139	-	unavailable	unavailable	unavailable	unsaid		unavailable
140	-	unavailable	unavailable	unavailable	unsaid		unavailable
141	-	unavailable	unavailable	unavailable	unsaid		unavailable
142	-	unavailable	unavailable	unavailable	corrected	The session fixed	unavailable
143	-	unavailable	unavailable	unavailable	unsaid		unavailable
144	01a0eba4-4aea-7b31-bc75-af790c595024	claude-sonnet-5-5	411991	completed	unsaid		standard
145	01a0eba6-54a0-7981-ad06-747e4d920845	Composer 2.5	293018	completed	unsaid		standard
146	01a0ebb5-0ce5-7e10-bd3f-d2ec424b65c0	claude-sonnet-5-5	1189712	completed	unsaid		standard
147	01a0ebc8-c4bf-7283-8ef9-b5a54f45478e	claude-opus-5-5	4018931	completed	finished by hand	died	standard
148	01a0f04b-4a85-7492-bd4f-693c4326fcb1	Composer 2.5	852059	completed	unsaid		standard
149	01a0f050-077e-7f11-8f36-4197eef43027	claude-sonnet-5-5	620615	completed	unsaid		standard
150	01a0f064-0a90-7103-a185-f2333cdba903	claude-opus-5-5	7746889	completed	unsaid		standard
151	01a0f0b0-fc29-7570-8cab-43662625c3ed; 01a0f0c3-b058-76c2-a83b-fd5af9f61899	claude-opus-5-5; sonnet	6384745	completed; completed	unsaid		standard; standard
152	01a0f07a-bbc3-7ee3-85c2-0101b93234f6	claude-sonnet-5-5	2653991	completed	unsaid		standard
153	01a0f064-dbd8-7e00-ab33-1bb51a20a8c6; 01a0f06e-531b-79f3-8630-60107004981c	gpt-5.6-sol-high; claude-sonnet-5-5	unavailable	failed; completed	finished by hand	died	standard; standard
154	01a0f077-772d-7e13-a5d7-2aca2838f99a	claude-sonnet-5-5	492378	completed	unsaid		standard
155	01a0f07c-1bbd-7110-9e0c-7eb0c2135eb4	claude-sonnet-5-5	662390	completed	unsaid		standard
156	01a0f08c-f3f8-75f1-b3a4-e081c734d087; 01a0f08d-1379-7ce1-af73-9a79ae05459f	gpt-6-astra; claude-opus-5-5	1622868	completed; completed	unsaid		standard; standard
157	01a0f0b1-1b02-76a0-848f-afe94c12b506	gemini-3.1-pro-high	986581	completed	unsaid		standard
158	01a0f0b1-3b13-7790-b73e-a631ba0b2dc5	Grok 4.7 256K High	3581124	completed	unsaid		standard
159	01a0f0bc-04eb-73e3-8c5a-1e1d3ab839d2	gpt-6.1-sol	379678	completed	unsaid		standard
160	01a0f0c6-2463-7ed1-8b6f-383e14a20e7d	claude-sonnet-5-5	1544608	completed	unsaid		standard
161	01a0f246-232e-7623-84d1-c7fd5d3cc924	claude-sonnet-5-5	1336574	completed	unsaid		standard
162	01a0f246-5ec9-7d13-8800-33fd6ef2ec5d	Grok 4.7 256K High	4799324	completed	unsaid		standard
163	01a0f2a5-705b-7751-b957-cfa1d44594f5	gemini-3.1-pro-high	2246581	completed	unsaid		standard
164	01a0f2c0-acd2-7450-97d8-8d4016455c97	Grok 4.7 256K High	1133638	completed	unsaid		standard
165	01a0f2c1-0e88-7bc0-8c3f-cf62fbc61f79	claude-sonnet-5-5	736377	completed	unsaid		standard
166	01a0f2c1-7090-7521-a787-5213cfbe67e9	claude-opus-5-5	1972233	completed	unsaid		standard
167	01a0f2c6-c137-78a1-b9eb-7347bb37259f	claude-opus-5-5	827190	completed	unsaid		standard
168	01a0f2c9-9881-7f20-b497-8c2e926c3e0d	claude-sonnet-5-5	759871	completed	unsaid		standard
169	01a0f2d9-2497-7982-87aa-b637cfd77aaf	claude-opus-5-5	6995682	completed	unsaid		standard
170	01a0f2d1-1e75-72f1-839f-5ba52297f810; 01a0f2d2-26a1-7ac3-83a8-eab6b500dcf1	claude-opus-5-5; claude-opus-5-5	2935903	completed; completed	unsaid		standard; standard
171	01a0f2e2-de3a-7310-992f-dc9fa3d689b2	claude-opus-5-5	2521081	completed	unsaid		standard
172	01a0f2ef-719a-7852-90a6-da2a5e98459f	claude-opus-5-5	4451516	completed	unsaid		standard
173	01a0f2e8-0fe7-7941-8c52-aab624e8a898	claude-opus-5-5	3080943	completed	unsaid		standard
174	01a0f307-7744-7bc3-ada8-e7bbd895a3a8	claude-opus-5-5	1005298	completed	unsaid		standard
175	-	unavailable	unavailable	unavailable	unsaid		unavailable
176	01a0f30a-e273-7382-b0b4-8e9508ecb1d2	claude-opus-5-5	1094250	completed	unsaid		standard
177	01a0f30e-4656-7d43-a6f8-e56dddf1c1b1	claude-opus-5-5	1723351	completed	unsaid		standard
178	01a0f312-404c-7a00-adc2-86f66a6fd43f	claude-opus-5-5	3207530	completed	unsaid		standard
179	01a0f317-da64-7d51-868e-357964a38ee2	claude-opus-5-5	869410	completed	unsaid		standard
180	01a0f31b-9349-7e31-8c94-0f132a32ecb2	claude-opus-5-5	1446291	completed	unsaid		standard
181	01a0f31d-e264-7d73-b81c-713a85ef734b	Grok 4.7 256K High	3302459	completed	unsaid		standard
182	01a0f346-2bed-7601-a963-e15235198367	claude-opus-5-5	3037591	completed	unsaid		standard
183	01a0f320-dd35-79f3-84e4-e7b9c5052414	claude-opus-5-5	2759257	completed	unsaid		standard
184	01a0f328-452b-7301-87e1-a1e0b75cde8b; 01a0f328-f1e8-7e73-a9cf-31f4cb280257	claude-opus-5-5; claude-opus-5-5	3787020	completed; completed	unsaid		standard; standard
185	01a0f36e-2656-7e90-ad57-99127ea2f287	Grok 4.7 256K High	4337291	completed	unsaid		standard
186	01a0f330-99db-7282-872b-65138ecdeace	claude-opus-5-5	2768309	completed	unsaid		standard
187	01a0f394-834a-79a2-aad2-8973f99cea08	claude-opus-5-5	2519597	completed	unsaid		standard
188	01a0f3bb-3b3d-7901-8a6f-880b9e41ad14	Grok 4.7 256K High	1876971	completed	unsaid		standard
189	01a0f3bb-6c3b-78c2-b738-c5f4cac1d5f2	claude-sonnet-5-5	990052	completed	unsaid		standard
190	01a0f3e3-2ef4-74e1-a241-e35de5d3f819	Grok 4.7 256K High	1698055	completed	unsaid		standard
191	01a0f3e3-7d64-7832-9d12-68f7d411b34b	claude-opus-5-5	2778770	completed	unsaid		standard
192	01a0f40f-e744-7361-ae2c-ae2c7b3a0825	claude-opus-5-5	2875475	completed	unsaid		standard
193	-	unavailable	unavailable	unavailable	unsaid		unavailable
194	-	unavailable	unavailable	unavailable	unsaid		unavailable
195	-	unavailable	unavailable	unavailable	unsaid		unavailable
196	-	unavailable	unavailable	unavailable	unsaid		unavailable
197	-	unavailable	unavailable	unavailable	unsaid		unavailable
198	-	unavailable	unavailable	unavailable	unsaid		unavailable
199	-	unavailable	unavailable	unavailable	unsaid		unavailable
200	-	unavailable	unavailable	unavailable	unsaid		unavailable
201	-	unavailable	unavailable	unavailable	unsaid		unavailable
202	-	unavailable	unavailable	unavailable	unsaid		unavailable
203	-	unavailable	unavailable	unavailable	unsaid		unavailable
204	-	unavailable	unavailable	unavailable	unsaid		unavailable
205	-	unavailable	unavailable	unavailable	unsaid		unavailable
206	-	unavailable	unavailable	unavailable	unsaid		unavailable
207	01a0f659-688b-70f3-bf65-1f26f2408244	claude-opus-5-5	4416478	completed	unsaid		standard
208	01a0f659-191b-7633-995f-15bb8c9707de	Composer 2.5	712140	completed	unsaid		standard
209	-	unavailable	unavailable	unavailable	unsaid		unavailable
210	-	unavailable	unavailable	unavailable	unsaid		unavailable
211	01a0f6c7-9574-7713-a41a-373e5d218c1f	Composer 2.5	2782896	completed	unsaid		standard
212	01a0f6c7-3395-7030-b487-ffb8c6a6c946	claude-opus-5-5	5449580	completed	unsaid		standard
213	01a0f6d9-5449-7972-be5d-1302ee056478	claude-opus-5-5	7032427	completed	unsaid		standard
214	-	unavailable	unavailable	unavailable	unsaid		unavailable
215	-	unavailable	unavailable	unavailable	unsaid		unavailable
216	-	unavailable	unavailable	unavailable	unsaid		unavailable
217	01a0f6d9-b63c-7ce1-ad42-85b0faffd62f	Composer 2.5	17269618	completed	unsaid		standard
218	-	unavailable	unavailable	unavailable	unsaid		unavailable
219	01a0f72d-5e6c-7e00-a63b-690b61ec89ba	gpt-6.1-sol	5747007	completed	unsaid		standard
220	01a0f74f-70c5-7000-8d78-e23c50ce1fb7	claude-opus-5-5	22005637	completed	unsaid		standard
221	-	unavailable	unavailable	unavailable	unsaid		unavailable
222	-	unavailable	unavailable	unavailable	unsaid		unavailable
223	-	unavailable	unavailable	unavailable	unsaid		unavailable
224	-	unavailable	unavailable	unavailable	unsaid		unavailable
225	-	unavailable	unavailable	unavailable	unsaid		unavailable
226	-	unavailable	unavailable	unavailable	unsaid		unavailable
227	-	unavailable	unavailable	unavailable	unsaid		unavailable
228	-	unavailable	unavailable	unavailable	unsaid		unavailable
229	-	unavailable	unavailable	unavailable	unsaid		unavailable
230	-	unavailable	unavailable	unavailable	unsaid		unavailable
231	-	unavailable	unavailable	unavailable	unsaid		unavailable
232	01a10644-6583-7251-a45f-91c8644e04e4	claude-opus-5-5	8197693	completed	unsaid		standard
233	01a1066b-7dc9-7350-a790-597bf2c77551	claude-opus-5-5	5603412	completed	unsaid		standard
234	01a10691-e77a-7612-baa9-bfd5e75aafe7	claude-opus-5-5	2359341	completed	unsaid		standard
235	01a1066b-7e06-7292-825a-ec54aca5bb7c	Grok 4.6 High	11617133	completed	unsaid		standard
236	01a10691-e755-7643-9671-592a83fed2a0	Grok 4.6 High	1908378	completed	unsaid		standard
237	01a10644-6546-7073-b2ac-8f038c48adf9	Grok 4.6 High	8449007	completed	unsaid		standard
238	01a1342b-7041-7493-aa38-d8235538b1d4; 01a1342f-10ac-7552-abf6-841768b3a40e	gpt-6.1-sol; gpt-6.1-sol	1077073	completed; completed	finished by hand	died	standard; standard
239	-	unavailable	unavailable	unavailable	unsaid		unavailable
240	01a1018e-49fe-7e72-ba86-15555d4d8738	Composer 2.5	3683852	completed	rejected	Not applied	standard
241	01a1019b-d792-7f81-9e14-70571dcf9f6d	Composer 2.5	947221	completed	unsaid		standard
242	01a1018f-20c1-7e11-9857-c86a1e6b6315; 01a0fd01-cc30-7e23-8808-5824363d9b45	claude-opus-5-5; claude-opus-5-5	14321768	completed; completed	unsaid		standard; standard
243	01a101a7-89a9-70b1-87d1-104fb5a1c5ef; 01a0fe73-5a9c (missing)	claude-opus-5-5; unavailable	unavailable	completed; missing	unsaid		standard; unavailable
244	01a1019b-62ac-7ab2-86b5-b1a48d5f384c; 01a0fe6e-995e (missing)	gpt-6.1-sol; unavailable	unavailable	completed; missing	unsaid		standard; unavailable
245	01a101a3-5835-7112-9214-f8f3377f1ab0	Composer 2.5	7624695	completed	unsaid		standard
246	01a101ae-3c68-73a1-a36a-2051a2782c28	Composer 2.5	2381401	completed	unsaid		standard
247	01a101a0-b0bd-7331-b6ad-f08d33c107ea	claude-opus-5-5	4052217	completed	unsaid		standard
248	01a101a0-b0ae-7030-9911-d78a694401b5	gpt-6.1-sol	1313701	completed	unsaid		standard
249	01a101b2-a68e-7183-84c9-13f0c43fcc7e	Composer 2.5	1392033	completed	unsaid		standard
250	01a101a9-6ff9-7591-b7b3-7cbf14b455b7	claude-opus-5-5	2212922	completed	unsaid		standard
251	01a10389-ec0e-71b0-ab2f-00f93083b7df	claude-opus-5-5	1449675	completed	unsaid		standard
252	01a10389-ec1d-74e0-a62a-2fb770ccefbb	gpt-6.1-sol	1173567	completed	unsaid		standard
253	01a103a7-f596-7a20-a4f0-c0dc0aef8845	Composer 2.5	1350313	completed	unsaid		standard
254	01a103a7-f5ce-7ca2-ad76-1cf580fd885e	gpt-6.1-sol	550843	completed	unsaid		standard
255	01a103a8-4b5c-7bd3-8829-0cac1a22145e; 01a103bd-e45e-76e2-8c81-e9c3a1e3df50; 01a103ea-de78-7542-90ed-1a1c0d8faac4	claude-opus-5-5; unavailable; unavailable	unavailable	completed; failed; completed	unsaid		standard; unavailable; unavailable
256	01a103b0-ec0d-7543-b7aa-8293b050a6ed; 01a103af-c365-70f2-99b4-e08e0f215cc5	gpt-6.1-sol; gpt-6.1-sol	4392652	completed; completed	unsaid		standard; standard
257	01a103af-c38c-71b0-ba99-ebab6c9f6e1e; 01a103b6-cbbb-7f51-8817-d02b860024bc	claude-opus-5-5; claude-haiku-4-5-20251001	3262428	completed; completed	unsaid		standard; standard
258	01a103b4-93ab-7fd0-871a-6bc38a76ef56	Composer 2.5	967632	completed	unsaid		standard
259	01a103b8-a3fd-75f0-ab8f-47cd3cd014b0	claude-opus-5-5	4429132	completed	unsaid		standard
260	01a10578-7d60-7012-afe5-70df6b64af6f	Grok 4.6 High	3069983	completed	unsaid		standard
261	-	unavailable	unavailable	unavailable	unsaid		unavailable
262	01a1044a-13b1-7391-a604-bfc5524633ba	gpt-6.1-sol	1334271	completed	unsaid		standard
263	01a1044a-13b1-7580-a20f-e1111c776e31	Composer 2.5	3579079	completed	unsaid		standard
264	01a106e2-809d-7363-b5c8-f56e0c09ae89	claude-opus-5-5	2223173	completed	unsaid		standard
265	01a104fd-8f5a-7f11-bfd7-0cdac71ddbd1	Grok 4.6 High	3078816	completed	unsaid		standard
266	01a1073a-63cb-78c0-9cf1-2f766d4ce2e6	claude-opus-5-5	6028882	completed	unsaid		standard
267	-	unavailable	unavailable	unavailable	unsaid		unavailable
268	01a104b1-23c5-7f12-88b4-33df286ac0f3	gpt-6.1-sol	1190618	completed	unsaid		standard
269	01a104ec-9311-7160-ad9f-301950c63b25	Grok 4.6 High	4839863	completed	unsaid		standard
270	01a104f8-d22b-71b1-b2ea-e9b602298d4a; 01a104eb-dbc3-7dd0-ab1b-ecac08aaae8a; 01a104eb-95c1-7bf3-8236-9a2826610f2c	gpt-6.1-sol; Grok 4.6 High; Grok 4.7 256K High	939390	completed; completed; completed	unsaid		standard; judge; judge
271	01a106b4-a233-74f1-8b53-14283570643f	claude-opus-5-5	4154506	completed	unsaid		standard
272	01a104f8-d22b-7772-94b3-dcecfec70852	claude-opus-5-5	7766705	completed	rejected	discarded	standard
273	01a106e2-8075-73e0-8787-3cb1b80da6d9	Grok 4.6 High	4957033	completed	corrected	amended by the session	standard
274	01a1073a-63cb-7e63-b549-412997958302	Grok 4.6 High	6355337	completed	unsaid		standard
275	01a1050b-c1bf-7150-9c69-275736a16756	claude-opus-5-5	11169542	completed	unsaid		standard
276	01a10517-84b9-73d0-82a6-52db0a98333a	Grok 4.6 High	972078	completed	unsaid		standard
277	01a10517-84cc-7761-8c98-51d5dc4de790; 01a0814a-c730-7e11-847b-fac691b5d57b	gpt-6.1-sol; unavailable	unavailable	completed; completed	unsaid		standard; unavailable
278	01a1052a-6599-7442-aefa-446ee719b52c	gpt-6.1-sol	924112	completed	unsaid		standard
279	01a1052f-22c1-7eb0-a5db-f3172d4db508	claude-opus-5-5	1594355	completed	unsaid		standard
280	01a10540-0ea7-7972-88eb-07ce3d97b934	Grok 4.6 High	4669037	completed	unsaid		standard
281	01a10553-f215-7483-a4fd-bd881ef50871; 01a1056a-5b22-7c12-9a85-ed97fbc9df4a	gpt-6.1-sol; claude-opus-5-5	3873951	completed; completed	rejected	rejected	standard; standard
282	01a1056a-8b37-7b02-968c-f910f74b4bfc	gpt-6.1-sol	627172	completed	unsaid		standard
283	01a10583-ffb6-7712-a39b-b4ac9a638827	Grok 4.6 High	2168550	completed	unsaid		standard
284	01a10540-0ea6-7db2-919d-624a244ddcdb	gpt-6.1-sol	1158423	completed	unsaid		standard
285	01a1051f-e4e1-7cd1-b384-fa47a820d70c	Grok 4.6 High	3520769	completed	unsaid		standard
286	01a10545-736f-7b83-8fac-816373cf0555; 01a10504-aeab-7973-80d3-88af31195095	claude-opus-5-5; claude-opus-5-5	52304843	completed; completed	unsaid		standard; standard
287	01a10566-33ba-7bb1-8c96-6543b8c71882	Grok 4.6 High	976356	completed	unsaid		standard
288	-	unavailable	unavailable	unavailable	unsaid		unavailable
289	01a1070d-abba-7f71-8051-457af17c4b88	Grok 4.6 High	5241960	completed	unsaid		standard
290	-	unavailable	unavailable	unavailable	corrected	amended by the session	unavailable
291	01a106b4-a233-70f0-9912-2ab4508b2190	Grok 4.6 High	6187279	completed	unsaid		standard
292	-	unavailable	unavailable	unavailable	unsaid		unavailable
293	01a133fa-e0cf-7653-b5a0-e27f81da38cd	gpt-6.1-sol	2235447	completed	unsaid		standard
294	-	unavailable	unavailable	unavailable	unsaid		unavailable
295	01a13402-9d49-7411-8f95-23835693536b	gpt-6.1-sol	1313790	completed	unsaid		standard
296	01a1340b-146a-7091-a15e-3b7ab34a5281	gpt-6.1-sol	3126661	completed	unsaid		standard
297	01a13405-c36a-7e30-b509-174ee94e03fa	gpt-6.1-sol	208651	completed	unsaid		standard
298	01a13433-9ca3-7991-8b9b-92428da157f1	gpt-6.1-sol	2729151	completed	unsaid		standard
299	01a1342d-0a38-7f12-9347-e448463988ae	gpt-6.1-sol	607366	completed	unsaid		standard
300	01a13431-b2d5-72b3-8842-f3b36f545e9a	gpt-6.1-sol	unavailable	failed	finished by hand	The session finished	standard
301	01a13438-d161-7bb2-ac17-a6f4cd2e6398	gpt-6.1-sol	1916258	completed	unsaid		standard
302	-	unavailable	unavailable	unavailable	unsaid		unavailable
303	-	unavailable	unavailable	unavailable	rejected	rejected	unavailable
304	-	unavailable	unavailable	unavailable	unsaid		unavailable
305	-	unavailable	unavailable	unavailable	unsaid		unavailable
306	-	unavailable	unavailable	unavailable	unsaid		unavailable
307	-	unavailable	unavailable	unavailable	unsaid		unavailable
308	-	unavailable	unavailable	unavailable	unsaid		unavailable
309	-	unavailable	unavailable	unavailable	unsaid		unavailable
310	-	unavailable	unavailable	unavailable	rejected	discarded	unavailable
311	-	unavailable	unavailable	unavailable	unsaid		unavailable
312	01a07308-a250-7792-9e8b-0d99697c99d9; 01a07218-8817-7581-9ddb-47565e5d5ba7; 01a073ef-701c-7522-b5bb-56d5ecba0f57; 01a07471-0a3e-74d2-b576-153ac0e7427b; 01a07891-a339-7173-96d9-70beb88e5efb; 01a07892-459d-7420-904e-91d6a6986159; 01a07915-c9cf-7cc0-b156-204771091a76; 01a07513-baa3-7cf0-b20a-ded11ede6a38; 01a078bc-420f-7b23-8e67-2dfc9f98f859; 01a07512-d36e-7651-b8d9-e9e279e5730b; 01a07eed-700a-7020-b503-1f5276f0cb15; 01a07eef-329c-7501-a201-d959a53c8777; 01a07eed-9c4a-7633-887d-e3c143af6dae; 01a07efe-97f4-7ae1-8ab7-308dfa14f439; 01a07efb-de49-7a70-81e4-76fa23c79d52; 01a07f02-36a9-7f21-95ed-5ddf27b5f317; 01a07f05-7a50-78b1-aaf1-48eaee7a75a4; 01a07f0d-2d90-7241-bef1-e96ef90a1012; 01a07f18-910f-7282-8bda-44347806e3db; 01a07f27-1683-72c3-946c-47885dd3f679; 01a07f18-9a28-7111-971c-f7e260c13cdc; 01a07f37-8a67-7542-bdd9-e4c2a630cfe7; 01a07f37-9461-7200-ad1a-4d4e50f9e395; 01a07f37-9cab-71c2-b114-73bb18b2b23a; 01a07f62-7061-7ea2-83ce-1c2e47c95c1d; 01a07ff5-2389-75f1-a3eb-7296adde0a9b; 01a07ffe-372a-7731-bab0-d46c03eaf0d9; 01a07f6c-ba21-7622-b5e1-813a9d9c60db; 01a07ffe-4142-7c40-8440-de8bbe6370f6; 01a07ffe-4808-7690-9b25-a72b854da243; 01a08008-ba33-7ef1-a142-82ad907cbf55; 01a0801d-7382 (missing); 01a08042-7cf1 (missing); 01a07f3f-3241-7231-9066-78838d46a73d; 01a0801d-7832 (missing); 01a0801d-7811 (missing); 01a08052-ccdc-7cc1-b28a-9c35de0bc986; 01a0800f-1fa4-7e92-8637-f6a4220d5d4e; 01a08053-3d85-7f40-97c9-059aed4da16c; 01a08127-6f58-7f22-9798-934a8cb414e5; 01a08129-de0c-73f0-aa74-927b6efd0dd9; 01a0812a-ad31-7810-ba8e-97ce189115f2; 01a0814a-c730-7e11-847b-fac691b5d57b; 01a08169-eb3b-7053-9d0c-797a62ad1a37; 01a0822e-bc48-7e31-b36e-c2d266dc9c44; 01a08241-831d-74a3-8ce5-122dd0c70a6f; 01a08067-548b-7223-8d7c-a2709100486e; 01a08111-1e5e-7051-a541-971996a7fc27; 01a08112-fe30-7560-bb0b-88a14f3d2e1c; 01a0811e-d401-7452-a242-52329b8150d1; 01a084d3-bdd3-7831-af07-339c253faf33; 01a09405-f451 (ambiguous); 01a0902c-8d66-7e10-80a6-2d36c7879118; 01a095e6-626f-7bf2-ab17-7074eb9d4674; 01a0bcdb-e6ed-7180-8a96-3674b960cbc8; 01a0c220-909f-7432-b1a7-8292767278e7; 01a0c220-ce0e-7433-b869-343b7b6e6c6b; 01a0c223-9ec2-72e0-b5d4-e29782d692b0; 01a0c222-4305-74f0-a378-2b6c57e03ea4; 01a0c231-6583-7c72-b10e-981a2cc0d6fb; 01a0a59c-c395-7423-87e5-3dadfb52a4e1; 01a0a56e-c71f-7783-bf25-2a6a16ece5ef; 01a0a56f-9a20-7c93-a618-d712c9a618db; 01a07514-ac48-7bd2-8838-ea6a16fd3e9d; 01a0a572-e4ab-7e70-bf9d-cbad277b6040; 01a0c4a2-d6b4-71a3-a2e5-f1eba83d74fa; 01a0c4b4-81ec-7752-ab39-451ff5dfc144; 01a0c4c9-9ad1-79a1-9734-8bbe0fc5dd6a; 01a0c4a6-cf6d-7620-99ff-9d28de8f0c73; 01a0c5dd-5e0a-7881-8a3c-bb30b7edbb6c; 01a0c61b-9816-71e3-83d1-f155aa710b47; 01a0c624-12c5-7653-a890-d8ba05bcdb83; 01a0c5dd-984e-7730-8d36-b3f4e3334763; 01a0c5dd-bf97-7793-870f-395526f53a88; 01a0c61d-0639-7580-aad9-4f471d6ae619; 01a0c61d-4289-7c01-9676-1d51f718c3ce; 01a0c62d-b98d-75e3-bf26-b789306aa17d; 01a0c6a5-be8c-7ea3-aa56-0772f8d6e819; 01a0c6a7-9315-7132-b87c-28804b7c8075; 01a0c61e-e2b6-70d1-9198-de7a1497d42a; 01a0c924-5609-70e3-97a7-546cdeb981c4; 01a0c924-7d46-7993-a0aa-fb8d64eec576; 01a0c924-a494-7e82-bad2-5384686a42dd; 01a0c9ca-1591-7901-8d8f-1526e7e50287; 01a0c955-f7a1-70d0-988b-c5b0780d04b2; 01a0c960-a3a5-7dc3-a450-939485eb869c; 01a0c960-c1f5-7b10-9016-6b6d29546ff5; 01a0c960-e174-72f3-8a7f-7dfc4dd20a10; 01a0c961-009d-7753-9134-25324c1115c4; 01a0c956-213e-7f80-908a-2c6c6e29c436; 01a0c956-462b-7542-924f-9bdf9d6c72f6; 01a0cc2a-d2a1-7b32-93ec-0d115885da73; 01a0cc4a-ad67-7c22-a5a1-9b2241d4f88e; 01a0cc32-1103-77e1-9db7-4317d5d07e0b; 01a0cc3f-1e0f-7b20-87c0-dc55f4c0ce62; 01a0cc2b-e29c-72a1-85a6-d13cc27eece8; 01a0cc2a-f20b-7d91-abb6-41f991c7be79; 01a0cc2b-10d6-7311-b935-de2988618d7a; 01a0cc3f-4438-7913-b52a-88c94518a149; 01a0cc32-3831-7d42-b307-0248751bb76c; 01a0cc32-5f53-7ee0-822c-fa6e8d5293fb; 01a0cc69-bbd2-7453-b4b6-cd2d8df859fe; 01a0cc9b-5397-7123-9e00-1ce4a51db97b; 01a0ccb1-2fea-7e13-a089-3930a184b5c8; 01a0ccb1-681d-7ab0-9e67-1dcb763c638e; 01a0ccb1-83c0-7761-a1d1-28d9a8b2c7cb; 01a0cf91-1d0b-7d02-b4fa-d2947cce86e7; 01a0cf8d-ce29-7cf0-9d96-5d924af9ce69; 01a0cf91-7525-7fa3-9ff1-bd67c9d66f29; 01a0cfa1-2185-7000-9ef6-4ced5d767e0f; 01a0cfd5-cdda-7182-915a-4a266342c74d; 01a0cfda-7012-7d43-93ab-224902b0701e; 01a0cfdb-e5c3-72e0-b965-185dcf5b6f8b; 01a0cfdc-ac67-7e63-a807-2333c7c271c6; 01a0cfec-faa6-7ad2-a7d3-7204f59b05d8; 01a0cff3-6576-7af0-b60b-7b59b18614be; 01a0cff3-8452-7f11-af24-aebd47904d41; 01a0cff3-a299-7161-b3d4-3a7566642a8e; 01a0cfa1-507d-7a61-99b1-18b7f5a30b0b; 01a0cf8d-1e06-7892-be75-aba714e94a7d; 01a0cfae-6b3a-7d62-a28e-09bee301c51f; 01a0cfa7-7c3e-7c50-8d2c-03c78e8c2f46; 01a0cc56-c15b-7ad2-9741-1dcce176e0a9; 01a0cf89-8b3d-77c1-9f7e-1b11c2a9a41b; 01a0cfcb-4fd1-77c2-b310-d1968148bd77; 01a0cfa7-d9c9-7f20-a8d1-b65328240674; 01a0d1fa-c301-7893-a2a5-019e63033a67; 01a0d02d-f812-7880-bd18-7ec35b213e0a; 01a0cc9d-8f15-7a00-a973-c2628fd9fa30; 01a0ca7d-f703-73b3-8598-a89d742d6ebf; 01a0c62b-d005-7180-8a65-6dc776facdac; 01a0d227-23ce-7833-85df-11b064b8d764; 01a0d226-d533-7ed3-ab50-f351c74a61ef; 01a0d227-ef37-7a22-b465-f53f807d0c09; 01a0d228-206c-7400-b190-07da21ebd542; 01a0d36f-1612-73e1-81ae-41805679498f; 01a0d36f-4447-7b22-b59b-af98eb122a01; 01a0d41b-c2cf-7a90-ac05-d94a965a1ba8; 01a0d41e-5b15-7420-8a07-dd925979f284; 01a0d41b-f171-7991-a9cb-0513a0fa9c79; 01a0d3e2-1098-78a2-bf8f-c5f8319a8311; 01a0d3e4-4245-70d2-807f-78986913a670; 01a0d3f5-50e5-7360-89fa-beef48855d14; 01a0d3e4-cd16-74e3-aa61-c77c8c24f3cd; 01a0d3ec-aba2-7aa3-8c0f-fe0d28d898db; 01a0d3ec-aba2-7352-940b-e3789394347c; 01a0d428-9ddf-7af2-85d5-cc948fa61616; 01a0d3f8-5272-7f20-95af-8e5e257e435c; 01a0d3f5-50e5-73b1-8726-55882f5caf91; 01a0d41e-8a6c-7be1-9fcb-42abb59e80fb; 01a0d425-68fa-7ad1-9eb3-af1fe207126b; 01a0d56c-a2b7-7552-9c8f-dcf65f4a3c71; 01a0d572-9dee-7743-8038-ad6ec119fbb9; 01a0d56c-a2b7-7491-b595-613283a958a1; 01a0d57a-ab27-74c2-bbdf-f620f1ca1597; 01a0d484-2d8c-7443-b46e-0e4f151c47e0; 01a0d56e-ba1b-7c72-8a44-0dc29389cae9; 01a0d587-e3e1-7523-9a7a-cfca460be192; 01a0d617-f5a3-7f61-9e76-8ddc4bb3101f; 01a0d618-3015-7f53-be11-90ab34c5eab3; 01a0d618-6b6e-7b73-9bd2-760f341247eb; 01a0d618-a6fd-7523-8a20-cefb63b79214; 01a0ded9-8e70-7ec2-87d3-564eb17c62db; 01a0ded9-c9a6-70f0-8d05-c8fd34921f35; 01a0e70c-f8cc-78f0-b91a-e8358ac208c1; 01a0e70c-f8e8-7bf1-9168-ef5bc83335fa; 01a0eba4-4aea-7b31-bc75-af790c595024; 01a0eba6-54a0-7981-ad06-747e4d920845; 01a0ebb5-0ce5-7e10-bd3f-d2ec424b65c0; 01a0ebc8-c4bf-7283-8ef9-b5a54f45478e; 01a0f04b-4a85-7492-bd4f-693c4326fcb1; 01a0f050-077e-7f11-8f36-4197eef43027; 01a0f064-0a90-7103-a185-f2333cdba903; 01a0f0b0-fc29-7570-8cab-43662625c3ed; 01a0f0c3-b058-76c2-a83b-fd5af9f61899; 01a0f07a-bbc3-7ee3-85c2-0101b93234f6; 01a0f064-dbd8-7e00-ab33-1bb51a20a8c6; 01a0f06e-531b-79f3-8630-60107004981c; 01a0f077-772d-7e13-a5d7-2aca2838f99a; 01a0f07c-1bbd-7110-9e0c-7eb0c2135eb4; 01a0f08c-f3f8-75f1-b3a4-e081c734d087; 01a0f08d-1379-7ce1-af73-9a79ae05459f; 01a0f0b1-1b02-76a0-848f-afe94c12b506; 01a0f0b1-3b13-7790-b73e-a631ba0b2dc5; 01a0f0bc-04eb-73e3-8c5a-1e1d3ab839d2; 01a0f0c6-2463-7ed1-8b6f-383e14a20e7d; 01a0f246-232e-7623-84d1-c7fd5d3cc924; 01a0f246-5ec9-7d13-8800-33fd6ef2ec5d; 01a0f2a5-705b-7751-b957-cfa1d44594f5; 01a0f2c0-acd2-7450-97d8-8d4016455c97; 01a0f2c1-0e88-7bc0-8c3f-cf62fbc61f79; 01a0f2c1-7090-7521-a787-5213cfbe67e9; 01a0f2c6-c137-78a1-b9eb-7347bb37259f; 01a0f2c9-9881-7f20-b497-8c2e926c3e0d; 01a0f2d9-2497-7982-87aa-b637cfd77aaf; 01a0f2d1-1e75-72f1-839f-5ba52297f810; 01a0f2d2-26a1-7ac3-83a8-eab6b500dcf1; 01a0f2e2-de3a-7310-992f-dc9fa3d689b2; 01a0f2ef-719a-7852-90a6-da2a5e98459f; 01a0f2e8-0fe7-7941-8c52-aab624e8a898; 01a0f307-7744-7bc3-ada8-e7bbd895a3a8; 01a0f30a-e273-7382-b0b4-8e9508ecb1d2; 01a0f30e-4656-7d43-a6f8-e56dddf1c1b1; 01a0f312-404c-7a00-adc2-86f66a6fd43f; 01a0f317-da64-7d51-868e-357964a38ee2; 01a0f31b-9349-7e31-8c94-0f132a32ecb2; 01a0f31d-e264-7d73-b81c-713a85ef734b; 01a0f346-2bed-7601-a963-e15235198367; 01a0f320-dd35-79f3-84e4-e7b9c5052414; 01a0f328-452b-7301-87e1-a1e0b75cde8b; 01a0f328-f1e8-7e73-a9cf-31f4cb280257; 01a0f36e-2656-7e90-ad57-99127ea2f287; 01a0f330-99db-7282-872b-65138ecdeace; 01a0f394-834a-79a2-aad2-8973f99cea08; 01a0f3bb-3b3d-7901-8a6f-880b9e41ad14; 01a0f3bb-6c3b-78c2-b738-c5f4cac1d5f2; 01a0f3e3-2ef4-74e1-a241-e35de5d3f819; 01a0f3e3-7d64-7832-9d12-68f7d411b34b; 01a0f40f-e744-7361-ae2c-ae2c7b3a0825; 01a0f659-688b-70f3-bf65-1f26f2408244; 01a0f659-191b-7633-995f-15bb8c9707de; 01a0f6c7-9574-7713-a41a-373e5d218c1f; 01a0f6c7-3395-7030-b487-ffb8c6a6c946; 01a0f6d9-5449-7972-be5d-1302ee056478; 01a0f6d9-b63c-7ce1-ad42-85b0faffd62f; 01a0f72d-5e6c-7e00-a63b-690b61ec89ba; 01a0f74f-70c5-7000-8d78-e23c50ce1fb7; 01a10644-6583-7251-a45f-91c8644e04e4; 01a1066b-7dc9-7350-a790-597bf2c77551; 01a10691-e77a-7612-baa9-bfd5e75aafe7; 01a1066b-7e06-7292-825a-ec54aca5bb7c; 01a10691-e755-7643-9671-592a83fed2a0; 01a10644-6546-7073-b2ac-8f038c48adf9; 01a1342b-7041-7493-aa38-d8235538b1d4; 01a1342f-10ac-7552-abf6-841768b3a40e; 01a1018e-49fe-7e72-ba86-15555d4d8738; 01a1019b-d792-7f81-9e14-70571dcf9f6d; 01a1018f-20c1-7e11-9857-c86a1e6b6315; 01a0fd01-cc30-7e23-8808-5824363d9b45; 01a101a7-89a9-70b1-87d1-104fb5a1c5ef; 01a0fe73-5a9c (missing); 01a1019b-62ac-7ab2-86b5-b1a48d5f384c; 01a0fe6e-995e (missing); 01a101a3-5835-7112-9214-f8f3377f1ab0; 01a101ae-3c68-73a1-a36a-2051a2782c28; 01a101a0-b0bd-7331-b6ad-f08d33c107ea; 01a101a0-b0ae-7030-9911-d78a694401b5; 01a101b2-a68e-7183-84c9-13f0c43fcc7e; 01a101a9-6ff9-7591-b7b3-7cbf14b455b7; 01a10389-ec0e-71b0-ab2f-00f93083b7df; 01a10389-ec1d-74e0-a62a-2fb770ccefbb; 01a103a7-f596-7a20-a4f0-c0dc0aef8845; 01a103a7-f5ce-7ca2-ad76-1cf580fd885e; 01a103a8-4b5c-7bd3-8829-0cac1a22145e; 01a103bd-e45e-76e2-8c81-e9c3a1e3df50; 01a103ea-de78-7542-90ed-1a1c0d8faac4; 01a103b0-ec0d-7543-b7aa-8293b050a6ed; 01a103af-c365-70f2-99b4-e08e0f215cc5; 01a103af-c38c-71b0-ba99-ebab6c9f6e1e; 01a103b6-cbbb-7f51-8817-d02b860024bc; 01a103b4-93ab-7fd0-871a-6bc38a76ef56; 01a103b8-a3fd-75f0-ab8f-47cd3cd014b0; 01a10578-7d60-7012-afe5-70df6b64af6f; 01a1044a-13b1-7391-a604-bfc5524633ba; 01a1044a-13b1-7580-a20f-e1111c776e31; 01a106e2-809d-7363-b5c8-f56e0c09ae89; 01a104fd-8f5a-7f11-bfd7-0cdac71ddbd1; 01a1073a-63cb-78c0-9cf1-2f766d4ce2e6; 01a104b1-23c5-7f12-88b4-33df286ac0f3; 01a104ec-9311-7160-ad9f-301950c63b25; 01a104f8-d22b-71b1-b2ea-e9b602298d4a; 01a104eb-dbc3-7dd0-ab1b-ecac08aaae8a; 01a104eb-95c1-7bf3-8236-9a2826610f2c; 01a106b4-a233-74f1-8b53-14283570643f; 01a104f8-d22b-7772-94b3-dcecfec70852; 01a106e2-8075-73e0-8787-3cb1b80da6d9; 01a1073a-63cb-7e63-b549-412997958302; 01a1050b-c1bf-7150-9c69-275736a16756; 01a10517-84b9-73d0-82a6-52db0a98333a; 01a10517-84cc-7761-8c98-51d5dc4de790; 01a1052a-6599-7442-aefa-446ee719b52c; 01a1052f-22c1-7eb0-a5db-f3172d4db508; 01a10540-0ea7-7972-88eb-07ce3d97b934; 01a10553-f215-7483-a4fd-bd881ef50871; 01a1056a-5b22-7c12-9a85-ed97fbc9df4a; 01a1056a-8b37-7b02-968c-f910f74b4bfc; 01a10583-ffb6-7712-a39b-b4ac9a638827; 01a10540-0ea6-7db2-919d-624a244ddcdb; 01a1051f-e4e1-7cd1-b384-fa47a820d70c; 01a10545-736f-7b83-8fac-816373cf0555; 01a10504-aeab-7973-80d3-88af31195095; 01a10566-33ba-7bb1-8c96-6543b8c71882; 01a1070d-abba-7f71-8051-457af17c4b88; 01a106b4-a233-70f0-9912-2ab4508b2190; 01a133fa-e0cf-7653-b5a0-e27f81da38cd; 01a13402-9d49-7411-8f95-23835693536b; 01a1340b-146a-7091-a15e-3b7ab34a5281; 01a13405-c36a-7e30-b509-174ee94e03fa; 01a13433-9ca3-7991-8b9b-92428da157f1; 01a1342d-0a38-7f12-9347-e448463988ae; 01a13431-b2d5-72b3-8842-f3b36f545e9a; 01a13438-d161-7bb2-ac17-a6f4cd2e6398	claude-opus-5[1m]; claude-sonnet-5; claude-opus-5[1m]; claude-opus-5[1m]; Composer 2.5; Composer 2.5; gemini-3.8-flash-low; claude-opus-5[1m]; claude-haiku-4-5-20251001; claude-haiku-4-5-20251001; Composer 2.5; claude-haiku-4-5-20251001; unavailable; claude-sonnet-5; gemini-3.1-pro-high; Composer 2.5; Cursor Grok 4.6 High Fast; claude-sonnet-5; claude-sonnet-5; claude-haiku-4-5-20251001; Composer 2.5; claude-haiku-4-5-20251001; claude-sonnet-5; claude-sonnet-5; Composer 2.5; haiku; Composer 2.5; haiku; Cursor Grok 4.6 High Fast; gemini-3.8-flash-high; Composer 2.5; unavailable; unavailable; claude-haiku-4-5-20251001; unavailable; unavailable; Cursor Grok 4.6 High Fast; Composer 2.5; Composer 2.5; unavailable; unavailable; claude-haiku-4-5-20251001; unavailable; unavailable; unavailable; unavailable; Cursor Grok 4.6 High Fast; Composer 2.5; Cursor Grok 4.6 High Fast; Composer 2.5; unavailable; unavailable; unavailable; claude-sonnet-5; Composer 2.5; unavailable; unavailable; unavailable; unavailable; unavailable; Composer 2.5; Cursor Grok 4.6 High Fast; Composer 2.5; unavailable; Composer 2.5; Composer 2.5; unavailable; unavailable; Composer 2.5; unavailable; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; unavailable; composer-2.5; unavailable; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; unavailable; Composer 2.5; Composer 2.5; unavailable; gemini-3.8-flash-high; claude-haiku-4-5-20251001; Composer 2.5; Composer 2.5; Composer 2.5; unavailable; Composer 2.5; Composer 2.5; unavailable; unavailable; claude-haiku-4-5-20251001; unavailable; unavailable; Composer 2.5; Composer 2.5; unavailable; claude-haiku-4-5-20251001; Composer 2.5; gemini-3.8-flash-low; unavailable; unavailable; Composer 2.5; claude-opus-5-5; unavailable; unavailable; unavailable; Composer 2.5; Composer 2.5; claude-haiku-4-5-20251001; Composer 2.5; gemini-3.8-flash-low; Composer 2.5; gemini-3.1-pro-high; unavailable; unavailable; claude-sonnet-5; claude-opus-5-5; claude-haiku-4-5-20251001; claude-opus-5-5; claude-haiku-4-5-20251001; unavailable; unavailable; unavailable; unavailable; claude-haiku-4-5-20251001; unavailable; Composer 2.5; gemini-3.8-flash-low; Composer 2.5; unavailable; Composer 2.5; Composer 2.5; unavailable; Composer 2.5; claude-opus-5-5; Composer 2.5; gemini-3.8-flash-high; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; unavailable; sonnet; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; Composer 2.5; claude-sonnet-5; claude-sonnet-5-5; Composer 2.5; claude-sonnet-5-5; claude-opus-5-5; Composer 2.5; claude-sonnet-5-5; claude-opus-5-5; claude-opus-5-5; sonnet; claude-sonnet-5-5; gpt-5.6-sol-high; claude-sonnet-5-5; claude-sonnet-5-5; claude-sonnet-5-5; gpt-6-astra; claude-opus-5-5; gemini-3.1-pro-high; Grok 4.7 256K High; gpt-6.1-sol; claude-sonnet-5-5; claude-sonnet-5-5; Grok 4.7 256K High; gemini-3.1-pro-high; Grok 4.7 256K High; claude-sonnet-5-5; claude-opus-5-5; claude-opus-5-5; claude-sonnet-5-5; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; Grok 4.7 256K High; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; Grok 4.7 256K High; claude-opus-5-5; claude-opus-5-5; Grok 4.7 256K High; claude-sonnet-5-5; Grok 4.7 256K High; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; Composer 2.5; Composer 2.5; claude-opus-5-5; claude-opus-5-5; Composer 2.5; gpt-6.1-sol; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; Grok 4.6 High; Grok 4.6 High; Grok 4.6 High; gpt-6.1-sol; gpt-6.1-sol; Composer 2.5; Composer 2.5; claude-opus-5-5; claude-opus-5-5; claude-opus-5-5; unavailable; gpt-6.1-sol; unavailable; Composer 2.5; Composer 2.5; claude-opus-5-5; gpt-6.1-sol; Composer 2.5; claude-opus-5-5; claude-opus-5-5; gpt-6.1-sol; Composer 2.5; gpt-6.1-sol; claude-opus-5-5; unavailable; unavailable; gpt-6.1-sol; gpt-6.1-sol; claude-opus-5-5; claude-haiku-4-5-20251001; Composer 2.5; claude-opus-5-5; Grok 4.6 High; gpt-6.1-sol; Composer 2.5; claude-opus-5-5; Grok 4.6 High; claude-opus-5-5; gpt-6.1-sol; Grok 4.6 High; gpt-6.1-sol; Grok 4.6 High; Grok 4.7 256K High; claude-opus-5-5; claude-opus-5-5; Grok 4.6 High; Grok 4.6 High; claude-opus-5-5; Grok 4.6 High; gpt-6.1-sol; gpt-6.1-sol; claude-opus-5-5; Grok 4.6 High; gpt-6.1-sol; claude-opus-5-5; gpt-6.1-sol; Grok 4.6 High; gpt-6.1-sol; Grok 4.6 High; claude-opus-5-5; claude-opus-5-5; Grok 4.6 High; Grok 4.6 High; Grok 4.6 High; gpt-6.1-sol; gpt-6.1-sol; gpt-6.1-sol; gpt-6.1-sol; gpt-6.1-sol; gpt-6.1-sol; gpt-6.1-sol; gpt-6.1-sol	unavailable	completed; completed; rate_limited; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; failed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; gated; completed; rate_limited; completed; failed; completed; missing; missing; completed; missing; missing; completed; completed; completed; failed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; ambiguous; completed; completed; completed; failed; failed; completed; failed; completed; completed; crashed; completed; skill_collision; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; failed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; skill_collision; completed; completed; completed; completed; completed; completed; completed; completed; completed; rate_limited; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; rate_limited; rate_limited; failed; gated; completed; completed; completed; completed; completed; completed; completed; completed; rate_limited; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; failed; failed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; failed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; missing; completed; missing; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; failed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; completed; failed; completed	rejected	rejected	standard; unavailable; read-only; read-only; standard; read-only; read-only; curate; standard; standard; standard; read-only; standard; standard; standard; standard; standard; standard; standard; read-only; standard; standard; standard; standard; standard; read-only; standard; read-only; standard; standard; standard; unavailable; unavailable; standard; unavailable; unavailable; standard; read-only; standard; unavailable; unavailable; standard; unavailable; unavailable; unavailable; unavailable; standard; standard; standard; standard; unavailable; unavailable; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; unavailable; judge; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; read-only; read-only; read-only; standard; standard; standard; standard; standard; standard; standard; standard; standard; read-only; read-only; read-only; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; unavailable; standard; unavailable; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; unavailable; unavailable; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; judge; judge; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard; standard
313	-	unavailable	unavailable	unavailable	unsaid		unavailable

unsaid: 285 of 313 tickets
```

# 61 - Should a checkout and a data root be the same directory?

Type: grilling
Status: resolved
Blocked by:

## Question

`Layout::discover()` reads the passwd home and appends `hydra`. The install instructions put
the checkout there too, so on the machine Hydra was built on the two are one directory, and
every document has been written as though that were the design rather than a coincidence of
the install path.

A reviewer who had never seen the repository cloned it to `/tmp`, ran the first command of the
quick start, and was shown 197 runs that were not theirs. Asked whether they wanted the tool,
they said they could not tell, "because I could not tell whether running it would disturb
anything of mine." That doubt cost them more time than every other defect combined.

`HYDRA_ROOT` already moves everything and is now documented, and `hydra status` prints the root
it read. That is a patch on the symptom. The question underneath:

1. **Is the data root a property of the machine, or of the checkout?** A machine-wide root
   means one corpus for every clone, which is what Ouroboros wants: more records, one place.
   A per-checkout root means a clone is inert until you point it somewhere, which is what a
   newcomer expects and what makes a second checkout safe.
2. **If it stays machine-wide, what refuses the surprise?** A first run from a checkout that is
   not the root could refuse and name `HYDRA_ROOT`, or print what it is about to write into
   and continue. Refusing costs a step for the person who wanted it; continuing costs the
   surprise this ticket is about.
3. **Does `hydra doctor` exist?** The reviewer's ranked suggestion was one command reporting
   the root, the three vendor binaries and their versions against the versions the code was
   measured on, the policy set sha, and which protected configs are missing. That is four
   checks Hydra already does at spawn and never shows anyone.

Whichever way this goes, `docs/harness-spec.md` section 5 owns the answer, because it is the
section that names the directories.

## Answer

Resolved 2026-09-21 with the human, after a research run on `agy` (01a0c59d-21cc), which
recommended a per-checkout root; the human took the other reading with the session's reasons.

1. The data root is a property of the machine: one corpus under `~/hydra`, which is what rung one
   learns from and what the install already makes coincide with the checkout. That is one
   machine and one corpus, not many machines, and nothing about volume changes.
2. What refuses the surprise is a refusal: a launch from a checkout that is not the root, with
   `HYDRA_ROOT` unset, refuses before writing anything, names the root it would have written into
   and the variable that says so. With the variable set, to that root or another, it launches.
3. `hydra doctor` exists: the root and the policy path, the four vendor binaries and their versions
   against the ones the code was measured on, the policy set sha, and which protected configs are
   missing. The research found that spawn does not check versions or missing configs today, so
   `doctor` is a new check and not a display of an old one.

Harness spec section 5 owns the answer and the session amends it. Build ticket 67.

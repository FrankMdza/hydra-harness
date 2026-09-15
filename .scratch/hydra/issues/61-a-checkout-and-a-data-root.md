# 61 - Should a checkout and a data root be the same directory?

Type: grilling
Status: open
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

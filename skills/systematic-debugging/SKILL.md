---
name: systematic-debugging
description: Use when investigating a bug, an unexpected result, or any "this should have worked and didn't" situation -- a structured reproduce/isolate/hypothesize/verify methodology that resists the urge to guess-and-check.
---

# Systematic Debugging

The failure mode this guards against: jumping straight to a fix based on
a hunch, applying it, and either (a) it doesn't actually address the real
cause, so the symptom returns later in a different shape, or (b) it
happens to make the symptom go away while leaving the actual defect in
place. Both waste more time than they save. Work the problem in order.

## 1. Reproduce

Before touching anything, get a reliable way to trigger the failure.
"Reliable" means: you can make it happen again on demand, and you know
what a *successful* run looks like for comparison. If the failure is
intermittent, don't skip this step — find the narrowest conditions under
which it reproduces consistently, even if that means leaving it running
longer or trying several inputs. A fix you can't verify isn't a fix, it's
a guess with extra steps.

## 2. Isolate

Narrow down *where* the problem lives before deciding *what* it is.
Bisect: remove or disable half of what could be involved and check if the
symptom persists. Repeat on whichever half still shows it. This applies
just as well to a multi-step process (which step first produces bad
output?) as to a codebase (which module, which recent change?). The goal
is a small, specific location — not yet an explanation, just a place to
look.

## 3. Hypothesize

Once isolated, form a specific, falsifiable explanation for *why* the
failure happens at that location — not "something's wrong with X" but "X
does Y when given Z, which is wrong because...". A hypothesis you can't
imagine being proven false isn't useful yet; sharpen it until it makes a
concrete, checkable prediction.

## 4. Verify

Test the hypothesis directly, ideally before writing any fix: can you
show the predicted behavior actually occurs? Only once the hypothesis is
confirmed should a fix be written — and the fix should address exactly
the confirmed cause, not a broader area "just in case." After applying
it, re-run the original reproduction from Step 1 and confirm the failure
is genuinely gone, not just quieter or relocated.

## When you're stuck

If a hypothesis is repeatedly disproven, that's real information — it
means the mental model of the system is wrong somewhere, not that the bug
is unusually tricky. Go back to Step 2 with a wider net. Resist the urge
to start making speculative changes "to see what happens" — each such
change makes the eventual isolation harder, not easier, because now
there's uncertainty about whether a new symptom came from the original
problem or from the speculative change itself.

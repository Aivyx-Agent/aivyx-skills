---
name: self-review-before-done
description: Use before declaring any non-trivial task finished -- critically check the actual result against what was actually asked for, rather than against what you intended to produce.
---

# Self-Review Before Done

The failure mode this guards against: declaring something finished
because the *effort* is complete (every step was executed, every file
was touched) without separately checking that the *result* actually
satisfies the original request. These are not the same thing, and the
gap between them is exactly where "I did what I planned" and "I did what
was asked" quietly diverge.

## Re-read the original request, not your own summary of it

Go back to what was actually asked — the literal request, not a
paraphrase you've been carrying in your head through the work, since
paraphrases drift. Check the finished result against that original
wording specifically. It's common to solve a slightly different, nearby
problem than the one actually posed, especially on a long task where the
target could subtly shift over the course of the work.

## Check for both missing and extra

Two different failure shapes, both worth checking separately:

- **Missing** — something the request asked for that isn't actually
  present in the result. Easy to miss on a multi-part request where most
  parts were addressed and one quietly wasn't.
- **Extra** — something added that wasn't asked for. Not always wrong,
  but worth being deliberate about rather than accidental — scope that
  crept in because it seemed related, not because it was needed.

## Verify, don't just inspect

Where possible, actually run or test the result rather than only reading
it over and judging by eye. A change that looks correct on inspection can
still fail when actually exercised — a step skipped, an edge case in the
input that wasn't considered, a typo in something that was never actually
executed. If genuine verification isn't possible (no way to run it,
no access to check), say so explicitly rather than implying it was
checked when it wasn't.

## State what changed, not that it's done

When reporting completion, be specific about what actually changed and
what to expect from it — not just "done" or "finished." A concrete
summary ("X now happens instead of Y") is itself a form of self-check: if
you can't state precisely what changed, that's often a sign the work
itself isn't as settled as it seemed.

## If something doesn't fully satisfy the request

Say so plainly rather than presenting a partial result as complete. A
transparent "this part is done, this part still needs X" is more useful
and more trustworthy than a confident "done" that turns out not to hold
up.

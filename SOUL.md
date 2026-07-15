# SOUL

*Why the Ising Engine exists. Read this before the rules — the rules are only
this document made operational.*

---

## The thing we are actually building

Most optimization software is a tool: you bring a problem, it returns a solution,
and it forgets. It learns nothing across problems. The next user starts from zero.
The knowledge — *which algorithm behaves how, and why* — lives only in the heads
of a few experts and dies with the project.

The Ising Engine exists to change what an optimizer *is*. Underneath, yes, it is a
fast, correct Ising/QUBO solver. But the real object is a **system that does
science on optimization itself** — that runs experiments, records them forever,
finds patterns, forms mechanistic explanations, tries to break those
explanations, keeps the ones that survive, and transfers what it learned to
problems it has never seen. A solver that gets wiser the longer it runs.

We are not building a better hammer. We are building something that studies
hammers, understands why they work, and eventually designs new ones.

## The long-term mission

**Turn optimization from a craft into a cumulative science — and let a machine be
the scientist.**

Every run should leave the world knowing something it didn't. Not just "this
instance scored X," but "algorithms with this structure behave this way under
these conditions, and here is the mechanism, and here is the ablation that proves
it." Knowledge that compounds. A body of understanding that grows monotonically,
where each experiment is a permanent brick and no brick is ever thrown away.

## The ultimate goal

A **Research Foundation Model for optimization**: a single learned model that,
shown any combinatorial problem — MaxCut, BQP, SAT, TSP, scheduling, routing, or a
family that doesn't have a name yet — can predict which algorithms will behave how,
why, and how to configure them, because it was trained on millions of the
platform's own honest experiments.

We are far from it, and we say so plainly. Today: ~18,570 experiments; the model
we can honestly justify is a ridge regression, not a Transformer. The frontier is
8% built. But the seed is real — cross-family transfer already works (a model
trained on one family predicts on another with measurable skill). The path is not
"build the big model." The path is **grow the dataset until the big model is
earned by evidence.** Curiosity and the Orchestrator exist to generate the
informative experiments that get us there. First 500k, then a million, then more.

## The 5–10 year vision

- **A platform that runs itself.** Not a script you launch — a standing research
  operation. It decides what to study next by what it doesn't yet know, allocates
  its own compute, retrains its own models when they go stale, and writes down
  what it found, around the clock, without a human in the loop for the routine.

- **A Knowledge OS.** A living, queryable body of optimization knowledge:
  conditional, evidence-weighted, source-attributed facts; theories that survived
  ablation; a memory that recalls "we have seen a structure like this before, and
  here is what worked." Not a pile of logs — an *understanding* you can ask
  questions of.

- **An AI Scientist worthy of the name.** One that forms a hypothesis, designs an
  experiment to falsify it, runs it, and updates — Popperian, not persuasive. It
  is judged by what it *refutes*, not by what it claims. Its proudest outputs
  include "the operator I proposed is weak" and "the plans I evolved lose to the
  production solver." An engine that can be wrong out loud is the only kind that
  can be trusted when it is right.

- **A Research Platform others can build on.** Problem families plug in through one
  energy-exact bridge; new operators become usable the moment they're registered,
  chosen by capability, never by name; every result is reproducible from a seed.
  A substrate for a community of optimization science, not one project's private
  tool.

- **The Foundation Model, earned.** When — and only when — the data justifies it,
  the shared representation across families that lets one model reason about
  optimization the way a language model reasons about text.

## Inviolable engineering principles

These do not bend for a deadline, a benchmark, or a demo. They are the soul made
concrete.

1. **Correctness is sacred.** We never trade it for speed, ever. A fast wrong
   answer is worthless; a fast wrong answer that *looks* right is dangerous.

2. **Honesty over optimism.** We report what happened, including — especially —
   the failures. We never fabricate a number. When a capability isn't available,
   we skip it and say so, rather than invent a plausible result. A refutation is a
   first-class finding.

3. **Every result is reproducible.** Same seed, same trajectory, same energy.
   Determinism is not a feature; it is what makes a claim a claim instead of an
   anecdote.

4. **Knowledge is append-only.** History is evidence. We never overwrite or
   silently delete an experiment. What we believed yesterday, and why we changed
   our mind, is part of the record.

5. **The core is trustworthy because it is stable.** The read-only engine — the
   light sources everything else depends on — is not casually rewritten. We build
   *above* it. Trust is earned by not moving the ground.

6. **Claims require evidence, at the right scale.** No performance win without a
   bit-identical A/B measurement. No big model without the data to justify it. We
   grow the dataset before the architecture. Ambition never outruns proof.

7. **We build for the next reader.** The one who inherits this — human or
   machine — should find a system that explains *why*, keeps its own history, and
   can be trusted to have told the truth about itself.

---

*If a rule in `CLAUDE.md` ever seems to fight the work, come back here. The rules
exist to protect these principles. When they can't, the principles win, and the
rule was wrong.*

# YC Application — Fall 2026
## Ready-to-paste answers + notes

> **Rules for using this document**
> 1. Everything in `[BRACKETS]` is a fact only you know. Fill it in truthfully.
> 2. Never invent a user, a quote, a revenue number, or a benchmark result. YC checks.
> 3. Text marked **PASTE** is ready to submit. Text marked *Note* is guidance for you — do not paste it.
> 4. Do the 10 customer interviews in §CRITICAL before you submit. That is the single
>    change that moves this from "impressive project" to "fundable company."

---

# CRITICAL — do this before you press Submit

The application has one fatal hole: **"How do you know people need what you're making?"**
Right now your honest answer is "I don't." That is the question YC weighs most heavily.

Fix it in 2 weeks, not 2 months:

1. List 30 people who own an expensive optimization problem: logistics/route planners,
   chip place-and-route engineers, grid dispatch operators, factory schedulers,
   quant portfolio teams, cloud capacity planners.
2. Email 30, expect 8 replies, book 10 calls. Ask three questions only:
   - What optimization problem costs you the most money today?
   - What do you solve it with now, and what does that cost you (license + engineer time)?
   - If an automated system found a better algorithm for it, what would you need to see
     before you'd run it in production?
3. Write down verbatim quotes. Put the two best in the "Idea" section.
4. If even one says "send me your thing," that person is your design partner. Name them
   in the application. One named design partner beats every technical achievement listed
   in this document.

---

# FOUNDERS

### Who writes code, or does other technical work on your product? Was any of it done by a non-founder?

**PASTE:**

> I write all of it. No non-founder has contributed code, design, or research.
>
> The system is ~[TOTAL] lines of Rust across two layers: a production solver
> (multi-spin-coded bit-slice engine with parallel tempering and population annealing)
> and a research platform of 28 modules that runs experiments against it. 280 tests,
> plus a golden-regression suite and a cross-backend equivalence check that fails the
> build if two independent implementations disagree on a single bit.
>
> The web interface is Next.js/TypeScript, also written by me.
>
> I use AI coding assistants heavily (Claude Code) as an accelerator, the same way I use
> a compiler — every architectural decision, every correctness invariant, and every
> experimental result is mine and is verified by the test suite before it lands.

*Note: disclose the AI tooling. Everyone uses it; hiding it and being asked about it in
the interview is far worse than stating it plainly.*

### Are you looking for a cofounder?

**PASTE:** `Yes`

**PASTE (if a text field appears):**

> Yes. I am strong on systems engineering and research methodology and weak on
> enterprise distribution. I am looking for a cofounder who has sold technical
> infrastructure into industrial or financial buyers, and who wants to own
> go-to-market while I own the engine.

*Note: ~10–11% of a YC batch is solo founders and the bar is explicitly higher for them.
Answering "Yes" costs you nothing and signals self-awareness. Answering "No" as a
first-time solo founder with no users reads as stubbornness.*

---

# FOUNDER VIDEO — 1 minute script

*Note: film on a phone, good light, no slides, look at the lens. Speak slowly — clarity
beats fluency. Practice until you can do it without reading.*

> "I'm [NAME], I'm [AGE], from [CITY].
>
> I spent the last [N] months building something unusual: a program that does science on
> algorithms by itself. It forms a hypothesis about why an optimization algorithm works,
> designs an experiment, runs it, and then tries to destroy its own explanation by
> ablation. If the explanation survives, it becomes a theory. If it doesn't, the failure
> is recorded permanently and never deleted.
>
> It has run 110,000 experiments so far. Every single one can be replayed bit-for-bit
> from its seed.
>
> The part I'm proudest of is the part that failed. The system proposed a new operator,
> tested it, and found it was worse than what it already had. That's recorded on the
> front page. I built it that way because a discovery engine that can't publish its own
> refutations is a marketing engine.
>
> Before this I [ONE SENTENCE: previous work / studies / what you built].
>
> What I want from YC is the thing I'm missing: getting this in front of people who have
> an optimization problem that costs them real money."

---

# COMPANY

### Company name

*Note: `Ising Engine` describes your physics substrate, not your business, and will box
you into a category that has proven commercially small. Pick something about discovery.*

Options:
- **Refutable** — leans on your differentiator, memorable, .com likely gettable
- **Ablate** — short, technical, verb
- **Provenance Labs**
- **Ising Labs** — if you want continuity with the repo

### Describe what your company does in 50 characters or less

*Note: count characters. Options with counts:*

- `AI that discovers optimization algorithms` — **41** ← recommended
- `An AI scientist that discovers algorithms` — **40**
- `AI that invents and proves solver algorithms` — **44**

### Company URL / product link / demo

- Company URL: `[your domain, or leave blank]`
- Product link: `[hosted Observatory URL — see the "when will you have a version" answer]`
- Demo video: see the shot list at the end of this document.

### What is your company going to make?

**PASTE:**

> We are building an autonomous scientist for algorithms.
>
> Most hard industrial problems — routing fleets, scheduling factories, placing circuits,
> dispatching power grids, rebalancing portfolios — reduce to the same mathematical shape:
> pick the best combination out of an astronomically large set. Today a company solves
> these by buying a general-purpose solver and paying expert engineers to hand-tune it
> for months. The tuning is craft, it is undocumented, and it leaves with the engineer.
>
> Our system replaces that craft with a machine that does it empirically. It generates a
> hypothesis about which algorithmic strategy suits a given problem structure, designs an
> experiment, runs it on a deterministic runtime, measures the result with real statistics,
> mines a rule from it, and then attempts to falsify that rule by ablation — deliberately
> breaking the mechanism to see whether performance actually collapses. Rules that survive
> become theories. Rules that die are recorded as refutations and never deleted.
>
> Two properties make this different from AutoML or hyperparameter search.
>
> First, **determinism**. Every one of the 110,000 experiments we have recorded can be
> replayed bit-for-bit from its seed. A discovered algorithm is not a claim, it is a
> reproducible artifact with a provenance chain. That is what makes it deployable somewhere
> that an auditor will ask questions.
>
> Second, **falsification**. The system is built to publish its own failures. Our platform
> proposed a new operator, tested it, found it worse than the existing ones, and that
> refutation sits in the knowledge base permanently. Our own evolved algorithms currently
> lose 5 out of 5 head-to-head against our hand-built production solver, and we say so.
> A discovery system whose results you cannot check is worthless in production.
>
> What exists today: the engine, the research loop, 110,000 recorded experiments across 23
> benchmark instances, 14 operators, and one suggestive cross-family result — a policy
> trained on MaxCut problems ranks algorithms correctly on 9 of 12 unseen non-MaxCut
> problems. I want to be exact about that number: at n=12 it does not reach statistical
> significance, and it is currently the only direct evidence for the premise this company
> rests on.
>
> I know that because our own audit process recently retracted a second, stronger-looking
> piece of evidence for the same premise. We had been citing a rank correlation of +0.747
> from a held-out evaluation. An audit proved the metric was mathematically incapable of
> measuring what we cited it for — the model is additive, the held-out features are
> constant within each fold, and a rank-based score cannot see a constant offset, so a
> model that ignored the problem entirely would have scored the same. We removed it from
> our own roadmap rather than keep quoting a number that looked good.
>
> What we are building next: turning that loop from a command-line research tool into a
> product a customer's optimization team can point at their own problem and get back a
> tuned algorithm with the evidence for why it works.

### Where do you live now, and where would the company be based after YC?

**PASTE:** `[Your City, Your Country] / San Francisco, USA`

### Explain your decision regarding location.

**PASTE:**

> I currently live in [CITY]. I intend to relocate to San Francisco for the batch and to
> base the company there afterward. Our buyers — infrastructure, logistics, semiconductor,
> and quantitative finance teams — and the investors who understand deep-tech
> infrastructure are concentrated there, and I need to be in the room with them. I have no
> constraints keeping me in [CITY] and I am prepared to move for the batch.

*Note: YC is in-person and strongly prefers founders who will relocate. If you have a visa
problem, say so plainly in one sentence — they deal with this constantly and honesty here
is fine. Hiding it is not.*

---

# PROGRESS

### How far along are you?

**PASTE:**

> The engine works and has been generating research results for months. No users yet.
>
> Built and verified:
> - Production solver: bit-sliced multi-spin coding, 64 replicas per machine word,
>   parallel tempering, population annealing, roof-duality presolve.
> - Research platform: 28 modules covering hypothesis generation, experiment design,
>   statistics, rule mining, a knowledge graph of ~300 evidence-weighted facts, and an
>   ablation-based theory engine.
> - Three independent solver backends that are cross-validated bit-identical against each
>   other, so an optimization can never silently change a result.
> - 110,000 recorded experiments across 23 standard benchmark instances (G-Set), all
>   replayable from seed.
> - 280 tests, plus a golden-regression suite protecting the production solver.
> - An HTTP API that solves a submitted problem.
> - A web interface, partially complete: the evidence and knowledge workspaces read real
>   recorded data; the control surfaces are honestly labelled as not yet wired.
>
> Measured results, including the negative ones:
> - One suggestive cross-family result: a policy trained on MaxCut ranks algorithms on
>   unseen non-MaxCut problems correctly in 9 of 12 held-out comparisons. Against a
>   coin-flip null that is p ≈ 0.07 — suggestive, not significant, and labelled that way.
> - A retraction, from the same week: we had cited a held-out rank correlation of +0.747
>   as further evidence of transfer. Our own audit proved that metric is mathematically
>   invariant to the problem features — a model ignoring the problem entirely scores
>   identically — so it cannot evidence transfer at all. Removed from the claim, with the
>   reasoning recorded permanently.
> - The learned policy beats both the default and random algorithm selection 5 out of 5 on
>   held-out instances.
> - Our own evolved algorithms lose 5 out of 5 to our hand-built production solver at equal
>   compute budget. We claim no superiority for the research engine and say so publicly.
> - Our external benchmark against two open-source solvers is statistically underpowered
>   (n=3, p=0.25) and is labelled that way in the product rather than quoted as a win.
>
> What is missing is the part YC is for: a customer. I have talked to [N] people with real
> optimization problems so far and I am scheduling more.

*Note: fill in `[N]` honestly, even if it is 0 today. If you do the interviews first, this
sentence becomes your strongest, not your weakest.*

### How long have each of you been working on this? How much of that has been full-time?

**PASTE:**

> [N months] total, of which [N months] full-time. [One sentence on what you were doing
> before, and whether you are currently employed or studying.]

### What tech stack are you using?

**PASTE:**

> Engine: Rust — Rayon for data parallelism, Tokio and Axum for the HTTP layer, Criterion
> for benchmarking. Bit-level SIMD-oriented data layouts, sparse CSR representations,
> no allocation in hot paths.
>
> Interface: Next.js, React, TypeScript, Tailwind, Playwright for browser verification.
>
> AI models in the product: a local model tier via Ollama running qwen2.5-coder:7b, which
> generates research hypotheses on the machine with no data leaving it, and a cloud tier
> using the Anthropic Messages API for deeper periodic analysis. Both tiers degrade
> honestly — if a tier is unavailable, the system records a skip rather than fabricating
> an answer.
>
> AI coding tools I use to build: Claude Code, daily and heavily.
>
> The learned models inside the platform are deliberately small right now — ridge
> regression and small MLPs for performance prediction, algorithm selection, trajectory
> dynamics, and a world model. Scaling them up is gated on data volume, not ambition; our
> dataset is 27× short of the size where a larger architecture would be justified, and the
> product states that gap openly rather than shipping an oversized model.

### Are people using your product?

**PASTE:** `No`

### When will you have a version people can use?

**PASTE:**

> [DATE — commit to something 6–8 weeks out].
>
> The public version is a hosted interface where a user can submit their own problem,
> watch the discovery loop run against it, and get back a tuned algorithm together with
> its evidence and a command that reproduces the result exactly. The solving API already
> works; what remains is the control plane that lets a browser start and supervise a run
> instead of a terminal.

### Do you have revenue?

**PASTE:** `No`

### Previous batch / pivot

**PASTE:** `[First time applying — leave blank, or state the truth if you have applied before.]`

### Incubator / accelerator participation

**PASTE:** `[No — or the truth.]`

---

# IDEA

### Why did you pick this idea? Do you have domain expertise? How do you know people need it?

*Note: this is the most important answer in the application, and currently your weakest.
Do the interviews. Replace the bracketed section with real quotes.*

**PASTE:**

> I picked it because I kept hitting the same wall from the other side. I set out to build
> the fastest open-source CPU solver for this class of problem, and I got a good one. But I
> learned that raw solver speed was not the bottleneck — the bottleneck was that choosing
> and tuning the right algorithm for a specific problem is undocumented human craft. I was
> spending most of my time being a slow, forgetful search process. So I built a system to
> do that part instead, and made it record everything so it would not be forgetful.
>
> My domain expertise is in the implementation, and it is deep: bit-level solver
> engineering, statistical experiment design, and the reproducibility discipline that makes
> results trustworthy. I have written every line of a system that runs a closed scientific
> loop and I understand exactly where it is strong and where it is not, which is why the
> product publishes its own refutations.
>
> On whether people need it, I will be precise about what I know and what I don't.
>
> What I know: the underlying demand is not speculative. Organizations pay for
> general-purpose optimization software and then pay specialists to tune it, and that
> tuning knowledge is lost when the specialist leaves. I have one suggestive technical
> result for the premise that a system can accumulate reusable expertise instead of
> starting over per customer: knowledge learned on MaxCut ranks algorithms on unseen
> non-MaxCut problems 9 times out of 12. At n=12 that is not conclusive and I do not
> present it as such. I retracted a stronger-sounding number of my own last week after
> proving it could not support the claim I was making with it.
>
> What I don't know yet, and am currently finding out: which industry feels this pain
> sharply enough to pay first. I have spoken with [N] practitioners in [DOMAINS].
> [QUOTE 1 — verbatim, with the person's role.]
> [QUOTE 2 — verbatim, with the person's role.]
> [If you have a design partner: "X at Y has agreed to give me their [problem] as a first
> test case."]
>
> I would rather tell you I am mid-way through finding this out than pretend I already
> know. Finding the first customer is the specific thing I want the batch for.

### Who are your competitors? What do you understand that they don't?

**PASTE:**

> Three groups.
>
> **Classical solver vendors** — Gurobi, CPLEX, FICO Xpress. They are excellent and
> entrenched, and they are not competitors so much as the environment. They sell a solver
> and leave tuning to you. We are not trying to beat Gurobi at being Gurobi; a discovered
> algorithm can call one.
>
> **Specialized annealing hardware and quantum-inspired solvers** — D-Wave, Fujitsu Digital
> Annealer, Toshiba SQBM+, and open-source implementations like OpenJij and neal. This
> category is a warning, not a target. D-Wave has spent more than a billion dollars over
> two decades and reported $24.6M of revenue in fiscal 2025 and $2.9M in Q1 2026. Selling
> a faster annealer is not a business, and I am not building that company.
>
> **AI-for-science labs** — Periodic Labs, Lila Sciences, CuspAI, FutureHouse, and
> DeepMind's FunSearch and AlphaEvolve work. These validate the category: machine-driven
> discovery produces real results. They are pointed at materials and biology.
>
> Here is what I understand that the third group's structure prevents them from acting on.
>
> **Algorithm discovery is the only science whose laboratory is a CPU.** Every one of those
> labs is bottlenecked on the physical experiment — a synthesis run costs money, takes days,
> and cannot be repeated identically. That is why they are raising hundreds of millions of
> dollars: they are buying robotic laboratories. My experiment costs microseconds, costs
> effectively nothing at the margin, and can be repeated bit-for-bit. I can run more
> experiments before lunch than a wet lab runs in a year. The data-volume problem that
> gates every foundation model for science gets solved in this domain first, and it gets
> solved by whoever built the loop that generates the data, not by whoever has the most GPUs.
>
> The second thing: **everyone optimizes for the answer; the barrier to deployment is trust
> in the answer.** An automated system that hands an engineer a novel algorithm and says
> "this is better, trust me" does not get deployed in a power grid or a trading system. One
> that hands over an algorithm plus the seed that reproduces it exactly, the ablation
> showing which component is actually causal, and the list of hypotheses it tried and
> refuted, does. I built determinism and falsification in from the first commit, not as
> compliance features but because I did not believe my own results otherwise. That turns
> out to be the thing that makes the output sellable.

### How do or will you make money? How much could you make?

*Note: YC prefers bottom-up arithmetic over a giant TAM slide. Keep it concrete.*

**PASTE:**

> Discovery engagements first, platform licence second.
>
> **Phase 1 — paid discovery engagements.** A customer gives us one problem they already
> solve and their current best result. We run the loop against it and deliver a tuned
> algorithm plus its full evidence and reproduction recipe. Priced at $25–75k per
> engagement. This is deliberately services-shaped: it forces us to learn what actually
> matters, and each engagement adds permanently to the knowledge base, so the marginal
> cost of the next customer in the same industry falls.
>
> **Phase 2 — annual platform licence.** Self-serve, on-premise or private cloud, for teams
> that want to point the loop at their own problems continuously. $80–250k per year
> depending on scale, which sits below the fully-loaded cost of the specialist engineer it
> replaces and alongside what these teams already pay for solver licences.
>
> **Arithmetic.** 40 platform customers at an average $150k is $6M ARR. 200 at $200k is
> $40M ARR. Teams that run continuous large-scale optimization — logistics networks,
> semiconductor design, grid operators, exchanges, cloud capacity planners — number in the
> low thousands globally, so a few hundred customers is a realistic ceiling for this
> product on its own.
>
> **The larger outcome, if the premise holds.** One result so far suggests accumulated
> algorithmic knowledge is reusable across problem types, at a sample size too small to
> settle it. If it holds up as the dataset grows, the asset stops being a tool and becomes
> a model that predicts which
> algorithm will work on a problem it has never seen. That is a different order of business
> and I am not going to claim it before the data supports it — our dataset is currently 27×
> short of where I would even test the hypothesis, and the product displays that gap.

### Other ideas you considered

**PASTE:**

> 1. **A commercial high-performance solver.** Sell the production engine directly — it is
>    fast and it exists. I did not lead with this because the category's economics are
>    visibly poor and it competes head-on with entrenched incumbents.
> 2. **Reproducibility infrastructure for algorithm research** — deterministic replay,
>    append-only experiment provenance, and automated ablation as a service for ML and
>    systems research groups. Real pain, but academia does not pay.
> 3. **Inference-scheduling optimization for LLM serving** — batching, placement, and
>    routing across heterogeneous accelerators is exactly the combinatorial shape the engine
>    is built for, and the buyers have large budgets and immediate pain. This is the
>    vertical I would test first if you told me to pick one tomorrow.

---

# EQUITY

- **Have you formed ANY legal entity yet?** `[Yes/No]`
- **If not, planned breakdown:**

**PASTE:**

> No entity yet. Planned at incorporation: 100% to me as sole founder and CEO, with a
> standard 4-year vest and 1-year cliff, and a 10% option pool reserved for early
> employees. If I bring on a cofounder before or during the batch, I intend to offer a
> substantial and near-equal share — I would rather have a real partner than a large slice
> of nothing.

- **Have you taken any investment yet?** `[No]`
- **Are you currently fundraising?** `[No]`

---

# CURIOUS

### What convinced you to apply to Y Combinator?

**PASTE:**

> I have built the hard technical half of a company alone and I know precisely what I am
> missing: I have never sold anything, and I have no distribution. That is the exact gap
> YC is built to close, and it is not a gap I can code my way out of.
>
> [Add the truth: who encouraged you, whether you have watched YC content / Startup School,
> whether you know any alumni.]

### How did you hear about Y Combinator?

**PASTE:** `[The truth — Hacker News, a friend, Startup School, etc.]`

---

# BATCH PREFERENCE

**PASTE:** `Fall 2026`

---

# DEMO VIDEO — 3 minute shot list

*Note: screen recording with your voice. No slides, no logo animation. Show the software
doing something real. If a surface is not wired up, do not show it.*

1. **0:00–0:20** — One sentence of what it is. Then straight into the terminal: start a
   campaign against a benchmark instance.
2. **0:20–1:00** — Show it actually running. The hypothesis it formed, the experiment it
   designed, the result. Say the numbers out loud.
3. **1:00–1:40** — The ablation. This is your centrepiece. Show it taking one of its own
   rules, deliberately breaking the mechanism, and measuring whether performance collapses.
   Explain in one sentence why a claim that survives this is different from a claim that
   was never tested.
4. **1:40–2:10** — The refutation ledger. Show the operator your system proposed, tested,
   and found worse — and that it is recorded permanently. Say: "this is the failure my own
   system published about itself."
5. **2:10–2:40** — Determinism. Take a record from the 110,000, copy its reproduce command,
   run it, and show the identical energy. This is the moment that convinces a technical
   reviewer.
6. **2:40–3:00** — What you need next, in one honest sentence: a first customer.

---

# What I would fix before submitting, in priority order

1. **Ten customer conversations.** Nothing else in this document moves the needle as much.
   Two real quotes in the Idea section changes the entire read.
2. **Name one design partner** if you can get one, even unpaid, even informal.
3. **Get the hosted version live** so the "product link" field is not empty. A reviewer who
   can click and see 110,000 real experiments with provenance is a reviewer you have
   already half-convinced.
4. **Rehearse the video out loud twenty times.** The interview is ten minutes of rapid
   questions in English. Fluency under pressure is trainable in two weeks and it is worth
   more than another feature.
5. **Keep every negative result in.** The refutations are not a weakness in this
   application — they are the only thing in it that a hundred other applicants cannot copy.

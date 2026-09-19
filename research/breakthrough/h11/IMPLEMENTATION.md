# H11 implementation plan — CD001

One standalone standard-library Python witness.py, outside production. Compute
all x,z for b=1..6 by independent expanded QUBO and native Ising expressions;
compare with the square, enumerate exact minima/gaps, report response counts,
coefficient scale and scalar input count. Two ablations: replace powers of two
by all ones (rank remains1, scalar contexts collapse); remove interior pair
couplings with distinct thresholds (separable response bound). Use b=1..6 in
both; tie counts reported, deterministic smallest-mask representative used.

This is an analytic falsification witness already derived before execution,
not a blinded performance experiment. No throughput/speed/AI-benefit claim.
Before --run create a content freeze of this plan, MATH and implementation;
refuse existing output. Results under h11/cd001/. Source hash is integrity only,
not retrospective preregistration. Independent review checks the finished code.

🦾 [Superpowers] Socratic Design Spec:
1. GOAL: Add OR, NOT, NAND, NOR, MUX (Multiplexer), and Comparators (GreaterThan, LessThan, Equal) to LogicBuilder.
2. PURPOSE: Enable the engine to compile Solidity/Yul smart contracts into QUBO matrices for vulnerability scanning and MEV.
3. TDD: Write red tests in tests/test_logic.rs first.
4. IMPLEMENTATION: Add the penalty functions for each gate to src/compiler/logic_builder.rs.
   - NOT(x): Penalty = 2xy - x - y + 1 (where y is output)
   - OR(a,b,z): Penalty = a*b - 2a*z - 2b*z + a + b + z
   - MUX(s,a,b,z): Penalty = 2s*a - 2s*z - 2a*z + s + a + z (for s=1, a passes)

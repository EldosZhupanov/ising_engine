// Qualification oracle: use upstream parsing AND upstream full recomputation.
// No optimization, at most 256 states. Independent of the Python exporter.
#include <iomanip>
#include <iostream>
#include <vector>
#include "problem/qubo_instance.h"
#include "heuristics/qubo/qubo_simple_solution.h"

int main(int argc, char** argv) {
    if (argc != 2) return 2;
    QUBOInstance instance(argv[1]);
    int n = instance.get_size();
    if (n < 1 || n > 8) return 2;
    for (int mask = 0; mask < (1 << n); ++mask) {
        std::vector<int> state(n);
        for (int i = 0; i < n; ++i) state[i] = (mask >> i) & 1;
        QUBOSimpleSolution solution(instance, nullptr, state, 0.0);
        solution.PopulateFromAssignments();
        std::cout << mask << " " << std::setprecision(17) << solution.get_weight() << "\n";
    }
}

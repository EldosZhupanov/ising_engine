// MQ-MST2-001: explicit callback controls; original upstream search is unchanged.
#include <chrono>
#include <cmath>
#include <cstdlib>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <stdexcept>
#include <string>
#include "heuristics/qubo/palubeckis2004b.h"

static void witness(const QUBOSimpleSolution& s) {
    std::cout << "\"objective\":" << std::setprecision(17) << s.get_weight() << ",\"state\":[";
    const auto& x = s.get_assignments();
    for (size_t i=0; i<x.size(); ++i) { if (i) std::cout << ','; std::cout << x[i]; }
    std::cout << ']';
}

class Stream final : public QUBOCallback {
    using Clock = std::chrono::steady_clock;
    const std::string mode_;
    const std::string affinity_;
    const Clock::time_point start_;
    bool stopped_ = false;
    unsigned long long calls_ = 0;
public:
    Stream(const std::string& mode, const std::string& affinity)
        : mode_(mode), affinity_(affinity), start_(Clock::now()) {}
    bool Report(const QUBOSimpleSolution& s, bool, double runtime) override {
        ++calls_;
        const auto ns = std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now()-start_).count();
        if (mode_ == "first" || (mode_ == "third" && calls_ >= 3)
            || (mode_ == "deadline" && ns >= 20000000)) stopped_ = true;
        const bool keep = !stopped_;
        std::cout << "{\"kind\":\"callback\",\"index\":" << calls_ << ',';
        witness(s);
        std::cout << ",\"continue\":" << (keep ? "true" : "false")
                  << ",\"elapsed_ns\":" << ns << ",\"upstream_runtime\":" << std::setprecision(17) << runtime
                  << ",\"affinity\":\"" << affinity_ << "\"}" << std::endl;
        return keep;
    }
    bool Report(const QUBOSimpleSolution& s, bool improved, double runtime, int) override {
        return Report(s, improved, runtime);
    }
    void final(const QUBOSimpleSolution& s) const {
        std::cout << "{\"kind\":\"final\",";
        witness(s);
        std::cout << ",\"callbacks\":" << calls_ << ",\"stop_seen\":"
                  << (stopped_ ? "true" : "false") << "}" << std::endl;
    }
};

int main(int argc, char** argv) {
    try {
        if (argc != 4) return 2;
        const std::string mode(argv[3]);
        if (mode != "first" && mode != "third" && mode != "deadline" && mode != "watchdog") return 2;
        size_t used = 0;
        const std::string seed_text(argv[2]);
        const unsigned long seed = std::stoul(seed_text, &used);
        if (used != seed_text.size() || seed > 65535 || seed_text.empty() || seed_text[0]=='-') return 2;
        QUBOInstance qi(argv[1]);
        if (qi.get_size() < 1) return 2;
        std::ifstream proc("/proc/self/status");
        std::string line, affinity;
        while (std::getline(proc,line)) if (line.find("Cpus_allowed_list:")==0) {
            affinity=line.substr(line.find(':')+1);
            affinity.erase(0,affinity.find_first_not_of(" \t"));
        }
        if (affinity != "0") return 3;
        std::srand(static_cast<unsigned int>(seed));
        Stream stream(mode,affinity);
        Palubeckis2004bMST2 solver(qi,0.02,false,&stream);
        stream.final(solver.get_best_solution());
    } catch (const std::exception& e) {
        std::cerr << e.what() << std::endl;
        return 2;
    }
    return 0;
}

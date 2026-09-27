// Reuse the frozen serializer, never invoke its optimization entry point.
#define main mq_screen_unused_search_main
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wreturn-type"
#include "../mqlib_screen/stream.cpp"
#pragma GCC diagnostic pop
#undef main
#include <chrono>
#include <thread>
#include <fstream>
#include <string>
#include <vector>

int main(int argc, char** argv) {
    using Clock = std::chrono::steady_clock;
    using Ns = std::chrono::nanoseconds;
    const auto start = Clock::now();
    if (argc != 3) return 2;
    QUBOInstance qi(argv[1]);
    const int n = qi.get_size();
    // Explicit diagnostic option used only by exhaustive energy unit tests.
    const bool spectrum = std::string(argv[2]) == "spectrum";
    if (spectrum && n > 8) return 3;
    const int count = spectrum ? (1 << n) : 4;
    std::vector<double> energies;
    for (int k = 0; k < count; ++k) {
        std::vector<int> x(n);
        for (int i = 0; i < n; ++i)
            x[i] = spectrum ? ((k >> i) & 1) :
                (k == 0 ? 0 : k == 1 ? 1 : k == 2 ? i % 2 : int(i % 3 == 0));
        QUBOSimpleSolution s(qi, nullptr, x, 0.0);
        s.PopulateFromAssignments();
        energies.push_back(s.get_weight());
    }
    if (spectrum) {
        for (double e : energies) std::cout << std::setprecision(17) << e << std::endl;
        return 0;
    }
    std::ifstream proc("/proc/self/status");
    std::string line, affinity;
    while (std::getline(proc, line)) {
        if (line.find("Cpus_allowed_list:") == 0) {
            affinity = line.substr(line.find(':') + 1);
            affinity.erase(0, affinity.find_first_not_of(" \t"));
        }
    }
    const auto ready = std::chrono::duration_cast<Ns>(Clock::now()-start).count();
    const auto delay = std::stoll(argv[2]);
    if (delay < 0) return 4;
    const auto before = Clock::now();
    std::this_thread::sleep_for(Ns(delay));
    const auto slept = std::chrono::duration_cast<Ns>(Clock::now()-before).count();
    QUBOSimpleSolution zero(qi, nullptr, 0);
    zero.PopulateFromAssignments();
    Stream stream;
    stream.Report(zero, true, 0.0);
    std::cout << "{\"kind\":\"diagnostic\",\"energies\":[";
    for (int k=0; k<4; ++k) { if (k) std::cout << ','; std::cout << std::setprecision(17) << energies[k]; }
    std::cout << "],\"ready_ns\":" << ready << ",\"sleep_ns\":" << slept
              << ",\"affinity\":\"" << affinity << "\"}" << std::endl;
    while (true) std::this_thread::sleep_for(std::chrono::seconds(1));
}

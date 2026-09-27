// Fixed upstream algorithms with identical publication and no-search controls.
#include <chrono>
#include <cstdlib>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <string>
#include <thread>
#include "heuristics/qubo/merz2002.h"
#include "heuristics/qubo/palubeckis2004b.h"

static void state(const QUBOSimpleSolution& s) {
    std::cout << "\"objective\":" << std::setprecision(17) << s.get_weight() << ",\"state\":[";
    const auto& x=s.get_assignments();
    for (size_t i=0;i<x.size();++i) { if (i) std::cout << ','; std::cout << x[i]; }
    std::cout << ']';
}
class Stream final : public QUBOCallback {
    bool emitted=false;
    double best=0;
public:
    bool Report(const QUBOSimpleSolution& s, bool, double) override {
        if (!emitted || s.get_weight()>best) {
            emitted=true; best=s.get_weight();
            std::cout << "{\"kind\":\"inc\",";state(s);std::cout << "}" << std::endl;
        }
        return true;
    }
    bool Report(const QUBOSimpleSolution& s,bool b,double t,int) override { return Report(s,b,t); }
};
int main(int argc,char** argv) {
    try {
        if (argc!=5) return 2;
        const std::string arm(argv[1]),mode(argv[4]),seed_text(argv[3]);
        if (arm!="mqlib_merz" && arm!="mqlib_mst2") return 2;
        if (mode!="search" && mode!="null_a" && mode!="null_b" && mode!="delay") return 2;
        size_t used=0;const unsigned long seed=std::stoul(seed_text,&used);
        if (used!=seed_text.size() || seed>65535 || seed_text.empty() || seed_text[0]=='-') return 2;
        QUBOInstance qi(argv[2]);
        std::ifstream proc("/proc/self/status");std::string line,affinity;
        while (std::getline(proc,line)) if (line.find("Cpus_allowed_list:")==0) {
            affinity=line.substr(line.find(':')+1);affinity.erase(0,affinity.find_first_not_of(" \t"));
        }
        if (affinity!="0") return 3;
        std::cout << "{\"kind\":\"ready\",\"affinity\":\"" << affinity << "\"}" << std::endl;
        std::srand(static_cast<unsigned>(seed));
        if (mode!="search") {
            QUBOSimpleSolution zero(qi,nullptr,0);
            const auto start=std::chrono::steady_clock::now();
            if (mode=="delay") std::this_thread::sleep_for(std::chrono::milliseconds(50));
            const auto ns=std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-start).count();
            std::cout << "{\"kind\":\"inc\",";state(zero);
            std::cout << ",\"sleep_ns\":" << ns << "}" << std::endl;
            for (;;) std::this_thread::sleep_for(std::chrono::seconds(1));
        }
        Stream stream;
        if (arm=="mqlib_merz") { Merz2002OneOpt solver(qi,3600.0,false,&stream); }
        else { Palubeckis2004bMST2 solver(qi,3600.0,false,&stream); }
    } catch (const std::exception& e) { std::cerr << e.what() << std::endl; return 2; }
    return 0;
}

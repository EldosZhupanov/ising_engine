// Emit native MQLib incumbents; outer process owns delivered-result deadline.
#include <cstdlib>
#include <iomanip>
#include <iostream>
#include "heuristics/qubo/merz2002.h"

class Stream final : public QUBOCallback {
    bool emitted=false;
public:
    bool Report(const QUBOSimpleSolution& s, bool improved, double) override {
        if (improved || !emitted) {
            emitted=true;
            std::cout << "{\"kind\":\"inc\",\"objective\":" << std::setprecision(17)
                      << s.get_weight() << ",\"state\":[";
            const auto& x=s.get_assignments();
            for (unsigned i=0;i<x.size();++i) { if (i) std::cout << ','; std::cout << x[i]; }
            std::cout << "]}" << std::endl;
        }
        return true; // The parent terminates this process at the common deadline.
    }
    bool Report(const QUBOSimpleSolution& s, bool b, double t, int) override { return Report(s,b,t); }
};

int main(int argc,char** argv) {
    if (argc!=3) return 2;
    char* end=nullptr; auto seed=std::strtoul(argv[2],&end,10);
    if (!end || *end || seed>65535) return 2;
    std::srand(static_cast<unsigned>(seed));
    QUBOInstance instance(argv[1]); Stream callback;
    Merz2002OneOpt solver(instance,3600.0,false,&callback);
}

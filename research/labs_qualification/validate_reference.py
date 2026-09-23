"""Check logging before TabuSearch returns and unchanged seeded search output."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile

from campaign import incumbent

DRIVER = r'''
#include "LABS.h"
#include "TabuSearch.h"
#include "timer.h"
int main(void) {
    Random rng = newRandom(0, 810001);
    TimerP timer = newTimer(); StartTimer(timer);
    Gene seq[20]; for (int i=0;i<20;i++) seq[i]=1;
    unsigned long long evaluations=0;
    for(int j=0;j<4;j++) {
        Fitness e=TabuSearch(seq,20,rng,timer,&evaluations);
        printf("RETURN %d ",e);
        for(int i=0;i<20;i++) putchar(seq[i]==1?'0':'1');
        printf(" %llu\n",evaluations);
    }
    return 0;
}
'''


def validate(original, instrumented):
    outputs = []
    with tempfile.TemporaryDirectory() as folder:
        temp = Path(folder)
        driver = temp / "driver.c"
        driver.write_text(DRIVER)
        for i, source in enumerate([original, instrumented]):
            binary = temp / f"driver{i}"
            subprocess.run(["gcc", "-O3", "-std=gnu99", "-D__RUN_IN_LINUX__",
                "-I" + str(source), "-I" + str(source / "libs/dcmt0.6.1/include"),
                str(driver), *[str(source / f) for f in
                    ["LABS.c", "TabuSearch.c", "random.c", "timer.c", "dynamicMem.c"]],
                str(instrumented / "libs/dcmt0.6.1/lib/libdcmt.a"),
                "-lm", "-lpthread", "-o", str(binary)], check=True, capture_output=True)
            outputs.append(subprocess.check_output([str(binary)], text=True, timeout=1))
    returns = [[line for line in out.splitlines() if line.startswith("RETURN ")] for out in outputs]
    assert returns[0] == returns[1] and len(returns[0]) == 4
    before_return = outputs[1].split("RETURN ", 1)[0]
    events = [incumbent(line, 20, 0, 1) for line in before_return.splitlines() if line.startswith("INC ")]
    assert len(events) > 1 and events[-1]["energy"] < events[0]["energy"]
    return {"status": "PASS", "unchanged_seeded_returns": returns[0],
            "incumbents_before_first_return": len(events), "outputs": outputs}


if __name__ == "__main__":
    print(json.dumps(validate(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()), indent=2))

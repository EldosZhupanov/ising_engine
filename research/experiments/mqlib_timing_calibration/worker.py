"""No-search, single-message worker. Input parsing is inside the measured cell."""
import json
import sys
import time


def main():
    with open(sys.argv[1]) as stream:
        model = json.load(stream)
    state = [0] * len(model['linear'])
    energy = model['offset'] + sum(h * x for h, x in zip(model['linear'], state))
    energy += sum(w * state[i] * state[j] for i, j, w in model['pairs'])
    delay_ns = int(sys.argv[2])
    before = time.monotonic_ns()
    if delay_ns:
        time.sleep(delay_ns / 1e9)
    after = time.monotonic_ns()
    event = {'state': state, 'energy': energy, 'sleep_start_ns': before,
             'sleep_end_ns': after, 'pre_emit_ns': time.monotonic_ns()}
    print(json.dumps(event, allow_nan=False), flush=True)
    while True:
        time.sleep(60)


if __name__ == '__main__':
    main()

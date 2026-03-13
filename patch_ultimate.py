import sys

with open('src/solver/ultimate.rs', 'r') as f:
    lines = f.readlines()

with open('src/solver/ultimate.rs', 'w') as f:
    for line in lines:
        if "final_state[i] = (field.get(i, 0, 0, 0) & 1) as i8;" in line:
            line = line.replace("final_state[i] = (field.get(i, 0, 0, 0) & 1) as i8;", "*item = (field.get(i, 0, 0, 0) & 1) as i8;")
        f.write(line)

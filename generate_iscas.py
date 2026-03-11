import random

NUM_KEYS = 16
LAYERS = 20
GATES_PER_LAYER = 50

print(f"--- ГЕНЕРАЦИЯ КРИПТО-КАСКАДА ---")

# 1. Генерируем секретный ключ
key_bits = [random.choice([0, 1]) for _ in range(NUM_KEYS)]
wire_values = {f"K_{i}": key_bits[i] for i in range(NUM_KEYS)}
current_layer_wires = [f"K_{i}" for i in range(NUM_KEYS)]

print(f"ОРИГИНАЛЬНЫЙ КЛЮЧ: {key_bits}")

gates = []
gate_counter = 0

# 2. Строим глубокую запутанную сеть (DAG)
for layer in range(LAYERS):
    next_layer_wires = []
    for g in range(GATES_PER_LAYER):
        # Выбираем два случайных провода из предыдущего слоя
        in1 = random.choice(current_layer_wires)
        in2 = random.choice(current_layer_wires)
        while in1 == in2 and len(current_layer_wires) > 1:
            in2 = random.choice(current_layer_wires)
        
        gate_type = random.choice(["AND", "XOR"])
        out_wire = f"N_{gate_counter}"
        ancilla = f"Anc_{gate_counter}"
        
        # Симулируем логику вперед, чтобы узнать правильный ответ
        v1 = wire_values[in1]
        v2 = wire_values[in2]
        if gate_type == "AND":
            out_val = v1 & v2
        else:
            out_val = v1 ^ v2
            
        wire_values[out_wire] = out_val
        gates.append((gate_type, in1, in2, out_wire, ancilla))
        next_layer_wires.append(out_wire)
        gate_counter += 1
        
    current_layer_wires = next_layer_wires

outputs = current_layer_wires

# 3. Записываем задачу для Rust-движка
with open("crypto_stress.txt", "w") as f:
    f.write("# ISCAS-STYLE CRYPTO STRESS TEST\n")
    f.write(f"# Depth: {LAYERS}, Total Gates: {gate_counter}\n\n")
    
    for g in gates:
        # Синтаксис нашего Rust-парсера: GATE TYPE IN1 IN2 OUT [ANCILLA]
        if g[0] == "AND":
            f.write(f"GATE {g[0]} {g[1]} {g[2]} {g[3]}\n")
        else:
            f.write(f"GATE {g[0]} {g[1]} {g[2]} {g[3]} {g[4]}\n")
            
    f.write("\n# ЗАМОРАЖИВАЕМ ВЫХОДЫ (ХЕШ)\n")
    for out_wire in outputs:
        # Переводим биты 0/1 в спины -1/1 для физического движка
        val = 1 if wire_values[out_wire] == 1 else -1
        f.write(f"CLAMP {out_wire} {val}\n")
        
    f.write("\n# ЦЕЛИ ДЛЯ ВЗЛОМА (ИСХОДНЫЙ КЛЮЧ)\n")
    for i in range(NUM_KEYS):
        f.write(f"TARGET K_{i}\n")

print(f"Сгенерирован файл 'crypto_stress.txt'.")
print(f"Всего логических вентилей: {gate_counter}")
print(f"Хеш (выходы) зафиксированы. Ключ скрыт.")

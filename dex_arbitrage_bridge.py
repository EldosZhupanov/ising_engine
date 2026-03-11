import math

TOKENS = ["ETH", "USDT", "DAI", "WBTC", "LINK"]
N = len(TOKENS)

rates = [[1.0 for _ in range(N)] for _ in range(N)]

# Искусственно делаем ЖИРНЫЙ арбитраж (очевидный градиент для Изинга)
rates[0][1] = 3000.0   # ETH -> USDT
rates[1][2] = 2.0      # USDT -> DAI (АНОМАЛИЯ ПУЛА! Дает x2 прибыли)
rates[2][0] = 1.0/3000.0 # DAI -> ETH

# Альтернативный маршрут (специально делаем убыточным)
rates[1][3] = 0.00005  # USDT -> WBTC
rates[3][0] = 10.0     # WBTC -> ETH (Убыток)

PENALTY = 1000.0 # Балансируем штраф

print("--- ГЕНЕРАЦИЯ QUBO ДЛЯ DEX АРБИТРАЖА (V2) ---")

with open("arbitrage_task.txt", "w") as f:
    f.write("# DEX ARBITRAGE TOPOLOGY\n")
    
    vars_dict = {}
    for i in range(N):
        for j in range(N):
            if i != j and rates[i][j] != 1.0:
                var_name = f"X_{i}_{j}"
                vars_dict[(i, j)] = var_name
                weight = -math.log(rates[i][j])
                # Масштабируем прибыль, чтобы движок ее "почувствовал"
                f.write(f"GATE ARB_LINEAR {var_name} {weight * 5.0}\n")
    
    f.write("\n# ОГРАНИЧЕНИЯ: ИДЕАЛЬНОЕ КОЛЬЦО ТРАНЗАКЦИЙ\n")
    for k in range(N):
        in_edges = [vars_dict[(i, k)] for i in range(N) if (i, k) in vars_dict]
        out_edges = [vars_dict[(k, j)] for j in range(N) if (k, j) in vars_dict]
        
        # Линейные штрафы за активацию любого ребра (x^2 = x)
        for edge in in_edges + out_edges:
            f.write(f"GATE ARB_LINEAR {edge} {PENALTY}\n")
            
        # Поощряем систему (-2*P), если на узле есть и вход, и выход (связка)
        for e_in in in_edges:
            for e_out in out_edges:
                f.write(f"GATE ARB_QUAD {e_in} {e_out} {-2.0 * PENALTY}\n")
                
        # Штрафуем (+2*P), если система пытается сделать "развилку" (два входа или два выхода)
        for i in range(len(in_edges)):
            for j in range(i+1, len(in_edges)):
                f.write(f"GATE ARB_QUAD {in_edges[i]} {in_edges[j]} {2.0 * PENALTY}\n")
        for i in range(len(out_edges)):
            for j in range(i+1, len(out_edges)):
                f.write(f"GATE ARB_QUAD {out_edges[i]} {out_edges[j]} {2.0 * PENALTY}\n")

    f.write("\n# ЖЕСТКАЯ ФИКСАЦИЯ: ИНИЦИИРУЕМ ТРАНЗАКЦИЮ ИЗ ETH В USDT\n")
    # Мы "пинаем" систему: заставляем ее начать сделку, фиксируя первый шаг.
    # Теперь она обязана найти путь домой в ETH, чтобы минимизировать энергию.
    f.write(f"CLAMP {vars_dict[(0, 1)]} 1\n")

    f.write("\n# TARGET VARIABLES\n")
    for var in vars_dict.values():
        f.write(f"TARGET {var}\n")

print("Файл 'arbitrage_task.txt' сгенерирован (с жесткими ограничениями маршрута).")

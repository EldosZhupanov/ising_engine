import requests
import json
import time

# Используем публичный эндпоинт Ethplorer (freekey дает отличные лимиты)
API_URL = "https://api.ethplorer.io/getTopTokenHolders/{}?apiKey=freekey&limit=30"

# Токены, где сидят самые жесткие киты (Можешь добавлять сюда адреса новых хайповых токенов)
TARGET_TOKENS = {
    "PEPE": "0x6982508145454ce325ddbe47a25d4ec3d2311933",
    "LINK": "0x514910771af9ca656af840dff83e8264ecf986ca"
}

def get_smart_money():
    print("=====================================================")
    print("🕵️  ЗАПУСК ИИ-ПАРСЕРА SMART MONEY (VIP WALLETS)")
    print("=====================================================\n")
    
    vip_wallets = set()

    for name, address in TARGET_TOKENS.items():
        print(f"📡 Взлом списка холдеров {name}...")
        url = API_URL.format(address)
        
        try:
            response = requests.get(url)
            if response.status_code == 200:
                data = response.json()
                if "holders" in data:
                    # Пропускаем Топ-3, так как обычно это пулы ликвидности (Биржи)
                    for holder in data["holders"][3:15]: 
                        wallet = holder["address"]
                        vip_wallets.add(wallet)
                        print(f"  [+] Найден Инсайдер: {wallet} (Доля: {holder['share']}%)")
            else:
                print(f"  [!] Ошибка API: {response.status_code}")
        except Exception as e:
            print(f"  [!] Ошибка соединения: {e}")
        
        time.sleep(2) # Анти-бан пауза

    print("\n✅ Сбор успешно завершен. Формируем инъекцию для Rust...")
    
    print("\n" + "👇 СКОПИРУЙ ЭТОТ БЛОК КОДА 👇")
    print("="*60)
    for wallet in list(vip_wallets)[:10]: # Выдаем 10 самых жирных китов
        print(f'top_traders.insert("{wallet}".parse::<Address>().unwrap());')
    print("="*60)
    print("👆 Вставь это в src/bin/scanner.rs вместо старого top_traders.insert(...) 👆")

if __name__ == "__main__":
    get_smart_money()

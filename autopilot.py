import subprocess
import requests
import time
import os
import xml.etree.ElementTree as ET

# --- CONNECTION CONFIG ---
TOKEN = os.environ.get("TELEGRAM_TOKEN")
CHAT_ID = os.environ.get("TELEGRAM_CHAT_ID")

if not TOKEN or not CHAT_ID:
    print("❌ Set TELEGRAM_TOKEN and TELEGRAM_CHAT_ID environment variables.")
    print("   See .env.example for reference.")
    exit(1)

def send_telegram_message(text):
    url = f"https://api.telegram.org/bot{TOKEN}/sendMessage"
    payload = {"chat_id": CHAT_ID, "text": text, "parse_mode": "HTML", "disable_web_page_preview": True}
    try:
        requests.post(url, json=payload)
    except Exception as e:
        print(f"❌ Telegram network error: {e}")

def get_latest_crypto_news():
    """Parse world news (RSS CoinTelegraph)"""
    print("🌍 Fetching news...")
    try:
        res = requests.get("https://cointelegraph.com/rss", timeout=5)
        root = ET.fromstring(res.content)
        news_list = []
        for item in root.findall('./channel/item')[:3]:
            title = item.find('title').text
            news_list.append(f"📰 <b>{title}</b>")
        return "\n".join(news_list)
    except Exception as e:
        return f"<i>News temporarily unavailable ({e})</i>"

def get_token_price(symbol):
    """Lookup token price via DexScreener API"""
    try:
        if symbol == "GLOBAL_UNI_V3": return "N/A"
        res = requests.get(f"https://api.dexscreener.com/latest/dex/search?q={symbol}", timeout=5).json()
        if res.get("pairs"):
            price = res["pairs"][0].get("priceUsd", "N/A")
            return f"${price}"
        return "Not found on DEX"
    except:
        return "API error"

def run_qubo_and_alert():
    print("\n🧠 [Autopilot] Querying QUBO engine...")
    try:
        result = subprocess.run(["./target/release/qubo"], capture_output=True, text=True)
        output = result.stdout

        if "🟢 LONG" in output:
            parts = output.split("⚡ ТОРГОВЫЕ РЕКОМЕНДАЦИИ:\n")
            if len(parts) > 1:
                raw_signals = parts[1].strip().split("------------------------------------------------")
                
                enriched_signals = []
                for signal in raw_signals:
                    if "LONG:" in signal:
                        lines = signal.strip().split("\n")
                        symbol = lines[0].split("LONG:")[1].strip()
                        price = get_token_price(symbol)
                        enriched_signal = signal.replace(f"LONG: {symbol}", f"LONG: {symbol} 💎 Price: {price}")
                        enriched_signals.append(enriched_signal)

                news = get_latest_crypto_news()
                
                final_message = f"🚨 <b>SIGNAL DETECTED</b> 🚨\n\n<code>{''.join(enriched_signals)}</code>\n\n🌐 <b>GLOBAL NEWS:</b>\n{news}"
                send_telegram_message(final_message)
                print("✅ Signal + Prices + News sent!")
        else:
            print("🔴 Market rejected by AI. No signals.")

    except Exception as e:
        print(f"[!] Engine error: {e}")

if __name__ == "__main__":
    print("=====================================================")
    print("✈️  AUTOPILOT: PRICES + NEWS ACTIVATED")
    print("=====================================================")
    
    send_telegram_message("🤖 <b>AUTOPILOT ACTIVATED</b>\n\n✅ DexScreener prices connected\n✅ CoinTelegraph news connected\n\nWaiting for signals...")
    
    while True:
        run_qubo_and_alert()
        time.sleep(300) 

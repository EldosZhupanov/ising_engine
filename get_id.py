import requests
import os

TOKEN = os.environ.get("TELEGRAM_TOKEN")

if not TOKEN:
    print("❌ Set TELEGRAM_TOKEN environment variable.")
    print("   See .env.example for reference.")
    exit(1)

url = f"https://api.telegram.org/bot{TOKEN}/getUpdates"

try:
    response = requests.get(url).json()
    if response.get("ok") and response.get("result"):
        chat_id = response["result"][-1]["message"]["chat"]["id"]
        print("=========================================")
        print(f"✅ YOUR CHAT ID: {chat_id}")
        print("=========================================")
        print(f"\nSet this in your .env file:")
        print(f"TELEGRAM_CHAT_ID={chat_id}")
    else:
        print("❌ Telegram returned empty response.")
        print("Response:", response)
except Exception as e:
    print(f"Error: {e}")

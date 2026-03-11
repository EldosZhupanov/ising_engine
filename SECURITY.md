# Security

## Environment Variables

All secrets **must** be provided via environment variables. Never hardcode tokens or keys in source code.

| Variable | Used By | Description |
|----------|---------|-------------|
| `TELEGRAM_TOKEN` | `autopilot.py`, `get_id.py` | Telegram Bot API token |
| `TELEGRAM_CHAT_ID` | `autopilot.py` | Target chat for notifications |

Copy `.env.example` to `.env` and fill in your values:

```bash
cp .env.example .env
# Edit .env with your actual values
```

The `.gitignore` ensures `.env` is never committed.

## Attack Surface

- **Telegram Bot:** If `TELEGRAM_TOKEN` is leaked, an attacker can impersonate the bot. Rotate tokens immediately if compromised.
- **Ethereum Addresses:** Smart money addresses in `config/smart_money.json` are public data, not secrets, but should be kept current.
- **MEVExecutor.sol:** The Solidity contract uses `onlyOwner` for access control. Deploy with a secure wallet and consider using a multisig.
- **External API Calls:** `scanner.rs` connects to Ethereum RPC nodes. Use authenticated endpoints in production.

## Dependency Notes

- `ethers 2.0.14` is deprecated (replaced by `alloy`). Consider migrating for long-term support.
- `edition = "2024"` requires nightly Rust. Pin your toolchain version for reproducibility.

## Reporting Vulnerabilities

If you find a security issue, please report it privately rather than opening a public issue.

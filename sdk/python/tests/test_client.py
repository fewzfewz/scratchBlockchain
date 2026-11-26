from nebula import NebulaClient, Wallet


def test_client_strips_trailing_slash():
    client = NebulaClient("https://example.com/")
    assert client.base_url == "https://example.com"


def test_wallet_generate_and_hash():
    wallet = Wallet.generate()
    assert wallet.address.startswith("0x")
    assert len(wallet.address) == 42
    assert len(wallet.public_key) == 32

    tx = {
        "sender": list(bytes.fromhex(wallet.address[2:])),
        "nonce": 0,
        "public_key": list(wallet.public_key),
        "payload": [],
        "gas_limit": 21_000,
        "max_fee_per_gas": 1_000_000_000,
        "max_priority_fee_per_gas": 100_000_000,
        "chain_id": 1,
        "to": [0x11] * 20,
        "value": 1_000_000,
    }
    digest = Wallet.hash_tx(tx)
    assert len(digest) == 32


def test_client_helper_routes(monkeypatch):
    client = NebulaClient("https://example.com")
    calls = []

    def fake_request(method, path, body=None):
        calls.append((method, path, body))
        return {"ok": True}

    monkeypatch.setattr(client, "_request", fake_request)
    client.block(10)
    client.latest_block()
    client.tx("0xabc")
    client.tx_history("0x1234", limit=5)
    client.gas_price()
    client.faucet("0x1234", amount="100")

    assert calls == [
        ("GET", "/block/10", None),
        ("GET", "/block/latest", None),
        ("GET", "/tx/0xabc", None),
        ("GET", "/txs/1234?limit=5", None),
        ("GET", "/gas_price", None),
        ("POST", "/faucet/request", {"address": "0x1234", "amount": "100"}),
    ]

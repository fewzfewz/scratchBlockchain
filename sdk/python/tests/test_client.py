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

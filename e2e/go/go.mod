module send_currency_tx

go 1.18

require github.com/Constellation-Labs/metakit-sdk/packages/go v0.0.0

require (
	github.com/btcsuite/btcd/btcec/v2 v2.3.2 // indirect
	github.com/cyberphone/json-canonicalization v0.0.0-20231217050601-ba74d44ecf5f // indirect
	github.com/decred/dcrd/dcrec/secp256k1/v4 v4.0.1 // indirect
)

replace github.com/Constellation-Labs/metakit-sdk/packages/go => ../../packages/go

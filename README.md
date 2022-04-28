# Rust Blockchain

> Project currenly is WIP

*P.S:* It's just a learning project, some of decisions which are used in this may be is not enough better or evenly true - but I'll glad to hear your hints :)

### Checklist
A some little list of my tasks which I want to bring to live, *step by step*

- [x] Base blockchain model
- [x] A possible to simple HTTP interaction
- [x] Proof-Of-Work validation
- [x] Blockchain storing
- [x] Transactions
- [x] Wallets
- [x] Transactions signing and verifying
- [ ] Transcations memory pool
- [ ] Blockchain network 

## Other possible things
- Atomic swaps
- Simple Smart contracts engine


### Usage
1. `cargo build`
2. `cargo run`
3. Go to `localhost:8080`

### API

| Method | Route | Request | Description |
| ------ |:-------:|:-------:| ----------- |
| **GET** | / | | Show blockchain history |
| **POST** | / | { "address": "*wallet_address*" } | Create blockchain if it's not exists |
| **GET** | /coins/{address} | | Show coins balance of address |
| **POST** | /coins | { "from": "*sender_wallet*", "to": "*recipient_wallet*", "amount": *some_positive_number* } | Send coins to another wallet address |
| **GET** | /wallet | | Show your local wallets |
| **POST** | /wallet | | Generate new local wallet |
# Rust Blockchain

> Project currenly is WIP

*P.S:* It's just a learning project, some of decisions which are used in this may be is not enough better or evenly true - but I'll glad to hear your hints :)

### Checklist
A some little list of my tasks which I want to bring to live, *step by step*

- [x] Base blockchain model
- [x] A possible to simple HTTP interaction
- [x] Proof-Of-Work validation
- [x] Blockchain storing
- [ ] Transactions
- [ ] Addresses
- [ ] Blockchain network 
- [ ] *Something else?...*


### Usage
1. `cargo build`
2. `cargo run`
3. Go to `localhost:8080`

### API

| Method | Request | Description |
| ------ |:-------:| ----------- |
| **GET** | | Shows all blockchain blocks |
| **POST** | *body:* { payload: { *any json-valid data here* } } | generates new block and adds payload data to the blockchain |
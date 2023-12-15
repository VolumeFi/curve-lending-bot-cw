# Curve Lending Bot CosmWasm smart contract for Uniswap V2 on Paloma

This is a CosmWasm smart contract to manage Curve Lending Bot on a Curve Lending Bot smart contract on EVM chain written in Vyper.

Users can create a crvUSD loan by deposit their token into a Vyper smart contract on EVM chain.

A scheduler or script fetch the list from the Vyper smart contract and run add_collateral or repay function with the parameters via Compass-EVM.

And then, the Vyper smart contract will add collateral or repay to reduce risk of the loan.

## ExecuteMsg

### AddCollateral

Run `add_collateral` function on Vyper smart contract.

| Key                        | Type           | Description                     |
|----------------------------|----------------|---------------------------------|
| bot_info                   | Vec\<BotInfo\> | Array of data to add collateral |

### Repay

Run `repay` function on Vyper smart contract.

| Key                        | Type           | Description            |
|----------------------------|----------------|------------------------|
| bot_info                   | Vec\<BotInfo\> | Array of data to repay |

### SetPaloma

Run `set_paloma` function on Vyper smart contract to register this contract address data in the Vyper contract.

| Key | Type | Description |
|-----|------|-------------|
| -   | -    | -           |

    UpdateCompass { new_compass: String },

### Update*

Run `update_*` function on Vyper smart contract to register this contract address data in the Vyper contract.

| Key | Type | Description |
|-----|------|-------------|
| -   | -    | -           |

## QueryMsg

### GetJobId

Get `job_id` of Paloma message to run `multiple_withdraw` function on a Vyper smart contract.

| Key | Type | Description |
|-----|------|-------------|
| -   | -    | -           |

#### Response

| Key    | Type   | Description      |
|--------|--------|------------------|
| job_id | String | Job Id on Paloma |

## Structs

### BotInfo

| Key        | Type    | Description                           |
|------------|---------|---------------------------------------|
| bot        | String  | Bot address                           |
| collateral | String  | Collateral Token address on EVM chain |
| amount     | Uint256 | Collateral / crvUSD Token amount      |

use crate::contract::{execute, instantiate};
use crate::msg::{BotInfo, ExecuteMsg, InstantiateMsg, SwapInfo};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coins, Uint256};
use std::str::FromStr;

#[test]
fn simple_test() {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        retry_delay: 60,
        job_id: "test_job_id".to_string(),
        creator: "creator".to_string(),
        signers: vec![],
    };
    let info = mock_info("creator", &coins(1000, "ugrain"));
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    let info = mock_info("creator", &coins(1000, "ugrain"));

    let swap_info = SwapInfo {
        route: vec![
            "0xd533a949740bb3306d119cc777fa900ba034cd52".to_string(),
            "0x4ebdf703948ddcea3b11f675b4d1fba9d2414a14".to_string(),
            "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_string(),
            "0x0000000000000000000000000000000000000000".to_string(),
            "0x0000000000000000000000000000000000000000".to_string(),
            "0x0000000000000000000000000000000000000000".to_string(),
            "0x0000000000000000000000000000000000000000".to_string(),
            "0x0000000000000000000000000000000000000000".to_string(),
            "0x0000000000000000000000000000000000000000".to_string(),
            "0x0000000000000000000000000000000000000000".to_string(),
            "0x0000000000000000000000000000000000000000".to_string(),
        ],
        swap_params: vec![
            vec![
                Uint256::from_str("2").unwrap(),
                Uint256::from_str("1").unwrap(),
                Uint256::from_str("1").unwrap(),
                Uint256::from_str("3").unwrap(),
                Uint256::from_str("3").unwrap(),
            ],
            vec![
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
            ],
            vec![
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
            ],
            vec![
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
            ],
            vec![
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
                Uint256::from_str("0").unwrap(),
            ],
        ],
        amount: Uint256::from_str("190730534729458").unwrap(),
        expected: Uint256::from_str("39463797565").unwrap(),
        pools: vec![
            "0x4ebdf703948ddcea3b11f675b4d1fba9d2414a14".to_string(),
            "0x0000000000000000000000000000000000000000".to_string(),
            "0x0000000000000000000000000000000000000000".to_string(),
            "0x0000000000000000000000000000000000000000".to_string(),
            "0x0000000000000000000000000000000000000000".to_string(),
        ],
    };
    let bot_info = BotInfo {
        bot: "0xCe946BC3cC175D1aAa11f8872573452F1BCcbe4c".to_string(),
        swap_infos: vec![swap_info],
        collateral: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
        amount: Uint256::from_str("190730534729458").unwrap(),
    };
    let msg = ExecuteMsg::AddCollateral {
        bot_info: vec![bot_info],
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
}

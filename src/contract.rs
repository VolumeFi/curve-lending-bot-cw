#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetJobIdResponse, InstantiateMsg, Metadata, PalomaMsg, QueryMsg};
use crate::state::{State, STATE};
use cosmwasm_std::CosmosMsg;
use ethabi::{Contract, Function, Param, ParamType, StateMutability, Token, Uint};
use std::collections::BTreeMap;
use std::str::FromStr;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:limit-order-bot-univ2-cw";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        retry_delay: msg.retry_delay,
        job_id: msg.job_id.clone(),
        owner: info.sender.clone(),
        metadata: Metadata {
            creator: msg.creator,
            signers: msg.signers,
        },
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("job_id", msg.job_id))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<PalomaMsg>, ContractError> {
    match msg {
        ExecuteMsg::AddCollateral { bot_info } => {
            execute::add_collateral(deps, env, info, bot_info)
        }
        ExecuteMsg::Repay { bot_info } => execute::repay(deps, env, info, bot_info),
        ExecuteMsg::SetPaloma {} => execute::set_paloma(deps, info),
        ExecuteMsg::UpdateCompass { new_compass } => {
            execute::update_compass(deps, info, new_compass)
        }
        ExecuteMsg::UpdateBlueprint { new_blueprint } => {
            execute::update_blueprint(deps, info, new_blueprint)
        }
        ExecuteMsg::UpdateRefundWallet { new_refund_wallet } => {
            execute::update_refund_wallet(deps, info, new_refund_wallet)
        }
        ExecuteMsg::UpdateGasFee { new_gas_fee } => {
            execute::update_gas_fee(deps, info, new_gas_fee)
        }
        ExecuteMsg::UpdateServiceFeeCollector {
            new_service_fee_collector,
        } => execute::update_service_fee_collector(deps, info, new_service_fee_collector),
        ExecuteMsg::UpdateServiceFee { new_service_fee } => {
            execute::update_service_fee(deps, info, new_service_fee)
        }
    }
}

pub mod execute {
    use super::*;
    use crate::msg::BotInfo;
    use crate::state::{ADD_COLLATERAL_TIMESTAMP, REPAY_TIMESTAMP};
    use crate::ContractError::{AllPending, Unauthorized};
    use cosmwasm_std::Uint256;
    use ethabi::Address;

    pub fn add_collateral(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        bot_info: Vec<BotInfo>,
    ) -> Result<Response<PalomaMsg>, ContractError> {
        assert!(!bot_info.is_empty(), "empty bot_info");
        let state = STATE.load(deps.storage)?;
        if state.owner != info.sender {
            return Err(Unauthorized {});
        }
        #[allow(deprecated)]
        let contract: Contract = Contract {
            constructor: None,
            functions: BTreeMap::from_iter(vec![(
                "add_collateral".to_string(),
                vec![Function {
                    name: "add_collateral".to_string(),
                    inputs: vec![
                        Param {
                            name: "bots".to_string(),
                            kind: ParamType::Array(Box::new(ParamType::Address)),
                            internal_type: None,
                        },
                        Param {
                            name: "swap_infos".to_string(),
                            kind: ParamType::Array(Box::new(ParamType::Array(Box::new(
                                ParamType::Tuple(vec![
                                    ParamType::FixedArray(Box::new(ParamType::Address), 11),
                                    ParamType::FixedArray(
                                        Box::new(ParamType::FixedArray(
                                            Box::new(ParamType::Uint(256)),
                                            5,
                                        )),
                                        5,
                                    ),
                                    ParamType::Uint(256),
                                    ParamType::Uint(256),
                                    ParamType::FixedArray(Box::new(ParamType::Address), 5),
                                ]),
                            )))),
                            internal_type: None,
                        },
                        Param {
                            name: "collateral".to_string(),
                            kind: ParamType::Array(Box::new(ParamType::Address)),
                            internal_type: None,
                        },
                        Param {
                            name: "lend_amount".to_string(),
                            kind: ParamType::Array(Box::new(ParamType::Uint(256))),
                            internal_type: None,
                        },
                    ],
                    outputs: Vec::new(),
                    constant: None,
                    state_mutability: StateMutability::NonPayable,
                }],
            )]),
            events: BTreeMap::new(),
            errors: BTreeMap::new(),
            receive: false,
            fallback: false,
        };
        let mut token_bots: Vec<Token> = vec![];
        let mut token_swap_infos: Vec<Token> = vec![];
        let mut token_collateral: Vec<Token> = vec![];
        let mut token_lend_amount: Vec<Token> = vec![];
        for bot in bot_info {
            if let Some(timestamp) =
                ADD_COLLATERAL_TIMESTAMP.may_load(deps.storage, bot.bot.to_owned())?
            {
                if timestamp
                    .plus_seconds(state.retry_delay)
                    .lt(&env.block.time)
                {
                    token_bots.push(Token::Address(Address::from_str(bot.bot.as_str()).unwrap()));
                    let mut token_swap_info: Vec<Token> = vec![];
                    for swap_info in bot.swap_infos {
                        let mut token_swap_info_element: Vec<Token> = vec![];
                        let mut token_route: Vec<Token> = vec![];
                        for route in swap_info.route {
                            token_route
                                .push(Token::Address(Address::from_str(route.as_str()).unwrap()));
                        }
                        token_swap_info_element.push(Token::FixedArray(token_route));
                        let mut token_swap_params: Vec<Token> = vec![];
                        for swap_params in swap_info.swap_params {
                            let mut token_inner_swap_params: Vec<Token> = vec![];
                            for inner_swap_params in swap_params {
                                token_inner_swap_params.push(Token::Uint(Uint::from_big_endian(
                                    &inner_swap_params.to_be_bytes(),
                                )))
                            }
                            token_swap_params.push(Token::FixedArray(token_inner_swap_params));
                        }
                        token_swap_info_element.push(Token::FixedArray(token_swap_params));
                        token_swap_info_element.push(Token::Uint(Uint::from_big_endian(
                            &swap_info.amount.to_be_bytes(),
                        )));
                        token_swap_info_element.push(Token::Uint(Uint::from_big_endian(
                            &swap_info.expected.to_be_bytes(),
                        )));
                        let mut token_pools: Vec<Token> = vec![];
                        for pool in swap_info.pools {
                            token_pools
                                .push(Token::Address(Address::from_str(pool.as_str()).unwrap()));
                        }
                        token_swap_info_element.push(Token::FixedArray(token_pools));
                        token_swap_info.push(Token::Tuple(token_swap_info_element));
                    }
                    token_swap_infos.push(Token::Array(token_swap_info));
                    token_collateral.push(Token::Address(
                        Address::from_str(bot.collateral.as_str()).unwrap(),
                    ));
                    token_lend_amount.push(Token::Uint(Uint::from_big_endian(
                        &bot.amount.to_be_bytes(),
                    )));
                    ADD_COLLATERAL_TIMESTAMP.save(
                        deps.storage,
                        bot.bot.to_owned(),
                        &env.block.time,
                    )?;
                }
            } else {
                token_bots.push(Token::Address(Address::from_str(bot.bot.as_str()).unwrap()));
                let mut token_swap_info: Vec<Token> = vec![];
                for swap_info in bot.swap_infos {
                    let mut token_swap_info_element: Vec<Token> = vec![];
                    let mut token_route: Vec<Token> = vec![];
                    for route in swap_info.route {
                        token_route
                            .push(Token::Address(Address::from_str(route.as_str()).unwrap()));
                    }
                    token_swap_info_element.push(Token::FixedArray(token_route));
                    let mut token_swap_params: Vec<Token> = vec![];
                    for swap_params in swap_info.swap_params {
                        let mut token_inner_swap_params: Vec<Token> = vec![];
                        for inner_swap_params in swap_params {
                            token_inner_swap_params.push(Token::Uint(Uint::from_big_endian(
                                &inner_swap_params.to_be_bytes(),
                            )))
                        }
                        token_swap_params.push(Token::FixedArray(token_inner_swap_params));
                    }
                    token_swap_info_element.push(Token::FixedArray(token_swap_params));
                    token_swap_info_element.push(Token::Uint(Uint::from_big_endian(
                        &swap_info.amount.to_be_bytes(),
                    )));
                    token_swap_info_element.push(Token::Uint(Uint::from_big_endian(
                        &swap_info.expected.to_be_bytes(),
                    )));
                    let mut token_pools: Vec<Token> = vec![];
                    for pool in swap_info.pools {
                        token_pools.push(Token::Address(Address::from_str(pool.as_str()).unwrap()));
                    }
                    token_swap_info_element.push(Token::FixedArray(token_pools));
                    token_swap_info.push(Token::Tuple(token_swap_info_element));
                }
                token_swap_infos.push(Token::Array(token_swap_info));
                token_collateral.push(Token::Address(
                    Address::from_str(bot.collateral.as_str()).unwrap(),
                ));
                token_lend_amount.push(Token::Uint(Uint::from_big_endian(
                    &bot.amount.to_be_bytes(),
                )));
                ADD_COLLATERAL_TIMESTAMP.save(deps.storage, bot.bot.to_owned(), &env.block.time)?;
            }
        }
        if token_bots.is_empty() {
            Err(AllPending {})
        } else {
            let tokens = vec![
                Token::Array(token_bots),
                Token::Array(token_swap_infos),
                Token::Array(token_collateral),
                Token::Array(token_lend_amount),
            ];
            Ok(Response::new()
                .add_message(CosmosMsg::Custom(PalomaMsg {
                    job_id: state.job_id,
                    payload: Binary(
                        contract
                            .function("add_collateral")
                            .unwrap()
                            .encode_input(tokens.as_slice())
                            .unwrap(),
                    ),
                    metadata: state.metadata,
                }))
                .add_attribute("action", "add_collateral"))
        }
    }

    pub fn repay(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        bot_info: Vec<BotInfo>,
    ) -> Result<Response<PalomaMsg>, ContractError> {
        assert!(!bot_info.is_empty(), "empty bot_info");
        let state = STATE.load(deps.storage)?;
        if state.owner != info.sender {
            return Err(Unauthorized {});
        }
        #[allow(deprecated)]
        let contract: Contract = Contract {
            constructor: None,
            functions: BTreeMap::from_iter(vec![(
                "repay".to_string(),
                vec![Function {
                    name: "repay".to_string(),
                    inputs: vec![
                        Param {
                            name: "bots".to_string(),
                            kind: ParamType::Array(Box::new(ParamType::Address)),
                            internal_type: None,
                        },
                        Param {
                            name: "collateral".to_string(),
                            kind: ParamType::Array(Box::new(ParamType::Address)),
                            internal_type: None,
                        },
                        Param {
                            name: "repay_amount".to_string(),
                            kind: ParamType::Array(Box::new(ParamType::Uint(256))),
                            internal_type: None,
                        },
                    ],
                    outputs: Vec::new(),
                    constant: None,
                    state_mutability: StateMutability::NonPayable,
                }],
            )]),
            events: BTreeMap::new(),
            errors: BTreeMap::new(),
            receive: false,
            fallback: false,
        };
        let mut token_bots: Vec<Token> = vec![];
        let mut token_collateral: Vec<Token> = vec![];
        let mut token_repay_amount: Vec<Token> = vec![];
        for bot in bot_info {
            if let Some(timestamp) = REPAY_TIMESTAMP.may_load(deps.storage, bot.bot.to_owned())? {
                if timestamp
                    .plus_seconds(state.retry_delay)
                    .lt(&env.block.time)
                {
                    token_bots.push(Token::Address(Address::from_str(bot.bot.as_str()).unwrap()));
                    token_collateral.push(Token::Address(
                        Address::from_str(bot.collateral.as_str()).unwrap(),
                    ));
                    token_repay_amount.push(Token::Uint(Uint::from_big_endian(
                        &bot.amount.to_be_bytes(),
                    )));
                    REPAY_TIMESTAMP.save(deps.storage, bot.bot.to_owned(), &env.block.time)?;
                }
            } else {
                token_bots.push(Token::Address(Address::from_str(bot.bot.as_str()).unwrap()));
                token_collateral.push(Token::Address(
                    Address::from_str(bot.collateral.as_str()).unwrap(),
                ));
                token_repay_amount.push(Token::Uint(Uint::from_big_endian(
                    &bot.amount.to_be_bytes(),
                )));
                REPAY_TIMESTAMP.save(deps.storage, bot.bot.to_owned(), &env.block.time)?;
            }
        }
        if token_bots.is_empty() {
            Err(AllPending {})
        } else {
            let tokens = vec![
                Token::Array(token_bots),
                Token::Array(token_collateral),
                Token::Array(token_repay_amount),
            ];
            Ok(Response::new()
                .add_message(CosmosMsg::Custom(PalomaMsg {
                    job_id: state.job_id,
                    payload: Binary(
                        contract
                            .function("repay")
                            .unwrap()
                            .encode_input(tokens.as_slice())
                            .unwrap(),
                    ),
                    metadata: state.metadata,
                }))
                .add_attribute("action", "repay"))
        }
    }

    pub fn set_paloma(
        deps: DepsMut,
        info: MessageInfo,
    ) -> Result<Response<PalomaMsg>, ContractError> {
        let state = STATE.load(deps.storage)?;
        if state.owner != info.sender {
            return Err(Unauthorized {});
        }
        #[allow(deprecated)]
        let contract: Contract = Contract {
            constructor: None,
            functions: BTreeMap::from_iter(vec![(
                "set_paloma".to_string(),
                vec![Function {
                    name: "set_paloma".to_string(),
                    inputs: vec![],
                    outputs: Vec::new(),
                    constant: None,
                    state_mutability: StateMutability::NonPayable,
                }],
            )]),
            events: BTreeMap::new(),
            errors: BTreeMap::new(),
            receive: false,
            fallback: false,
        };
        Ok(Response::new()
            .add_message(CosmosMsg::Custom(PalomaMsg {
                job_id: state.job_id,
                payload: Binary(
                    contract
                        .function("set_paloma")
                        .unwrap()
                        .encode_input(&[])
                        .unwrap(),
                ),
                metadata: state.metadata,
            }))
            .add_attribute("action", "set_paloma"))
    }

    pub fn update_compass(
        deps: DepsMut,
        info: MessageInfo,
        new_compass: String,
    ) -> Result<Response<PalomaMsg>, ContractError> {
        let state = STATE.load(deps.storage)?;
        if state.owner != info.sender {
            return Err(Unauthorized {});
        }
        let new_compass_address: Address = Address::from_str(new_compass.as_str()).unwrap();
        #[allow(deprecated)]
        let contract: Contract = Contract {
            constructor: None,
            functions: BTreeMap::from_iter(vec![(
                "update_compass".to_string(),
                vec![Function {
                    name: "update_compass".to_string(),
                    inputs: vec![Param {
                        name: "new_compass".to_string(),
                        kind: ParamType::Address,
                        internal_type: None,
                    }],
                    outputs: Vec::new(),
                    constant: None,
                    state_mutability: StateMutability::NonPayable,
                }],
            )]),
            events: BTreeMap::new(),
            errors: BTreeMap::new(),
            receive: false,
            fallback: false,
        };

        Ok(Response::new()
            .add_message(CosmosMsg::Custom(PalomaMsg {
                job_id: state.job_id,
                payload: Binary(
                    contract
                        .function("update_compass")
                        .unwrap()
                        .encode_input(&[Token::Address(new_compass_address)])
                        .unwrap(),
                ),
                metadata: state.metadata,
            }))
            .add_attribute("action", "update_compass"))
    }

    pub fn update_blueprint(
        deps: DepsMut,
        info: MessageInfo,
        new_blueprint: String,
    ) -> Result<Response<PalomaMsg>, ContractError> {
        let state = STATE.load(deps.storage)?;
        if state.owner != info.sender {
            return Err(Unauthorized {});
        }
        let new_blueprint_address: Address = Address::from_str(new_blueprint.as_str()).unwrap();
        #[allow(deprecated)]
        let contract: Contract = Contract {
            constructor: None,
            functions: BTreeMap::from_iter(vec![(
                "update_blueprint".to_string(),
                vec![Function {
                    name: "update_blueprint".to_string(),
                    inputs: vec![Param {
                        name: "new_compass".to_string(),
                        kind: ParamType::Address,
                        internal_type: None,
                    }],
                    outputs: Vec::new(),
                    constant: None,
                    state_mutability: StateMutability::NonPayable,
                }],
            )]),
            events: BTreeMap::new(),
            errors: BTreeMap::new(),
            receive: false,
            fallback: false,
        };

        Ok(Response::new()
            .add_message(CosmosMsg::Custom(PalomaMsg {
                job_id: state.job_id,
                payload: Binary(
                    contract
                        .function("update_blueprint")
                        .unwrap()
                        .encode_input(&[Token::Address(new_blueprint_address)])
                        .unwrap(),
                ),
                metadata: state.metadata,
            }))
            .add_attribute("action", "update_blueprint"))
    }

    pub fn update_refund_wallet(
        deps: DepsMut,
        info: MessageInfo,
        new_compass: String,
    ) -> Result<Response<PalomaMsg>, ContractError> {
        let state = STATE.load(deps.storage)?;
        if state.owner != info.sender {
            return Err(Unauthorized {});
        }
        let update_refund_wallet_address: Address =
            Address::from_str(new_compass.as_str()).unwrap();
        #[allow(deprecated)]
        let contract: Contract = Contract {
            constructor: None,
            functions: BTreeMap::from_iter(vec![(
                "update_refund_wallet".to_string(),
                vec![Function {
                    name: "update_refund_wallet".to_string(),
                    inputs: vec![Param {
                        name: "new_compass".to_string(),
                        kind: ParamType::Address,
                        internal_type: None,
                    }],
                    outputs: Vec::new(),
                    constant: None,
                    state_mutability: StateMutability::NonPayable,
                }],
            )]),
            events: BTreeMap::new(),
            errors: BTreeMap::new(),
            receive: false,
            fallback: false,
        };

        Ok(Response::new()
            .add_message(CosmosMsg::Custom(PalomaMsg {
                job_id: state.job_id,
                payload: Binary(
                    contract
                        .function("update_refund_wallet")
                        .unwrap()
                        .encode_input(&[Token::Address(update_refund_wallet_address)])
                        .unwrap(),
                ),
                metadata: state.metadata,
            }))
            .add_attribute("action", "update_refund_wallet"))
    }

    pub fn update_gas_fee(
        deps: DepsMut,
        info: MessageInfo,
        new_gas_fee: Uint256,
    ) -> Result<Response<PalomaMsg>, ContractError> {
        let state = STATE.load(deps.storage)?;
        if state.owner != info.sender {
            return Err(Unauthorized {});
        }
        #[allow(deprecated)]
        let contract: Contract = Contract {
            constructor: None,
            functions: BTreeMap::from_iter(vec![(
                "update_gas_fee".to_string(),
                vec![Function {
                    name: "update_gas_fee".to_string(),
                    inputs: vec![Param {
                        name: "new_gas_fee".to_string(),
                        kind: ParamType::Uint(256),
                        internal_type: None,
                    }],
                    outputs: Vec::new(),
                    constant: None,
                    state_mutability: StateMutability::NonPayable,
                }],
            )]),
            events: BTreeMap::new(),
            errors: BTreeMap::new(),
            receive: false,
            fallback: false,
        };

        Ok(Response::new()
            .add_message(CosmosMsg::Custom(PalomaMsg {
                job_id: state.job_id,
                payload: Binary(
                    contract
                        .function("update_gas_fee")
                        .unwrap()
                        .encode_input(&[Token::Uint(Uint::from_big_endian(
                            &new_gas_fee.to_be_bytes(),
                        ))])
                        .unwrap(),
                ),
                metadata: state.metadata,
            }))
            .add_attribute("action", "update_gas_fee"))
    }

    pub fn update_service_fee_collector(
        deps: DepsMut,
        info: MessageInfo,
        new_service_fee_collector: String,
    ) -> Result<Response<PalomaMsg>, ContractError> {
        let state = STATE.load(deps.storage)?;
        if state.owner != info.sender {
            return Err(Unauthorized {});
        }
        let new_service_fee_collector_address: Address =
            Address::from_str(new_service_fee_collector.as_str()).unwrap();
        #[allow(deprecated)]
        let contract: Contract = Contract {
            constructor: None,
            functions: BTreeMap::from_iter(vec![(
                "update_service_fee_collector".to_string(),
                vec![Function {
                    name: "update_service_fee_collector".to_string(),
                    inputs: vec![Param {
                        name: "new_service_fee_collector".to_string(),
                        kind: ParamType::Address,
                        internal_type: None,
                    }],
                    outputs: Vec::new(),
                    constant: None,
                    state_mutability: StateMutability::NonPayable,
                }],
            )]),
            events: BTreeMap::new(),
            errors: BTreeMap::new(),
            receive: false,
            fallback: false,
        };

        Ok(Response::new()
            .add_message(CosmosMsg::Custom(PalomaMsg {
                job_id: state.job_id,
                payload: Binary(
                    contract
                        .function("update_service_fee_collector")
                        .unwrap()
                        .encode_input(&[Token::Address(new_service_fee_collector_address)])
                        .unwrap(),
                ),
                metadata: state.metadata,
            }))
            .add_attribute("action", "update_service_fee_collector"))
    }

    pub fn update_service_fee(
        deps: DepsMut,
        info: MessageInfo,
        new_service_fee: Uint256,
    ) -> Result<Response<PalomaMsg>, ContractError> {
        let state = STATE.load(deps.storage)?;
        if state.owner != info.sender {
            return Err(Unauthorized {});
        }
        #[allow(deprecated)]
        let contract: Contract = Contract {
            constructor: None,
            functions: BTreeMap::from_iter(vec![(
                "update_service_fee".to_string(),
                vec![Function {
                    name: "update_service_fee".to_string(),
                    inputs: vec![Param {
                        name: "new_service_fee".to_string(),
                        kind: ParamType::Uint(256),
                        internal_type: None,
                    }],
                    outputs: Vec::new(),
                    constant: None,
                    state_mutability: StateMutability::NonPayable,
                }],
            )]),
            events: BTreeMap::new(),
            errors: BTreeMap::new(),
            receive: false,
            fallback: false,
        };

        Ok(Response::new()
            .add_message(CosmosMsg::Custom(PalomaMsg {
                job_id: state.job_id,
                payload: Binary(
                    contract
                        .function("update_service_fee")
                        .unwrap()
                        .encode_input(&[Token::Uint(Uint::from_big_endian(
                            &new_service_fee.to_be_bytes(),
                        ))])
                        .unwrap(),
                ),
                metadata: state.metadata,
            }))
            .add_attribute("action", "update_service_fee"))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetJobId {} => to_json_binary(&query::get_job_id(deps)?),
    }
}

pub mod query {
    use super::*;

    pub fn get_job_id(deps: Deps) -> StdResult<GetJobIdResponse> {
        let state = STATE.load(deps.storage)?;
        Ok(GetJobIdResponse {
            job_id: state.job_id,
        })
    }
}

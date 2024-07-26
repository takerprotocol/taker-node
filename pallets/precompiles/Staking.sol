// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.0;
/**
* @title Pallet Staking Interface
* Address :  0x000000000000000000000000000000000000044f
*/

interface Staking {
    function validatorCount() external view returns (uint256);
    function stashAccount(address account) external view returns (address);
    function stakingLedger(address account) external view returns (uint256, uint256);
    function payee(address account) external view returns (address);
    function activeEra() external view returns (uint256);
    function erasStakers(uint256 era, address validator) external view returns (address [] memory, uint256 [] memory);
    function erasValidatorPrefs(uint256 era, address validator) external view returns (uint256, bool);
    function nominators(address account) external view returns (address [] memory);
    function eraValidatorReward(uint256 era, address validator) external view returns (uint256);
    function eraNominatorReward(uint256 era, address nominator) external view returns (uint256);
    function erasTotalStake(uint256 era) external view returns (uint256);

    /** @dev Try nominate some valitors
    * Selector:
    * @param bond_value the target amount balance want to staking
    * @param payment_destination the target rewards destination
    * @param validators the target validators want to nominate
    */
    function bondAndNominate(uint256 bond_value, uint256 payment_destination, address[] memory validators) external;

    /** @dev Try been validator
    * Selector:
    * @param bond_value the target amount balance want to staking
    * @param payment_destination the target rewards destination
    * @param commission the proportion validator get
    * @param can_nominated if the validator want to be nominated
    * @param session_keys the combine pubkey for the velidator
    * @param proof the proof about the session keys
    */
    function bondAndValidate(uint256 bond_value, uint256 payment_destination, uint256 commission, bool can_nominated, bytes memory session_keys, bytes memory proof) external;

    /** @dev Try been validator
    * Selector:
    */
    function chill() external;

    /** @dev Try been validator
    * Selector:
    * @param bond_value the target amount balance want to staking from stash free balance
    */
    function bondExtra(uint256 bond_value) external;

    /** @dev Try been validator
    * Selector:
    * @param unbond_value the target amount balance want to staking
    */
    function unbond(uint256 unbond_value) external;


    /** @dev Try been validator
    * Selector:
    * @param vilidator the validator stash account
    * @param era_index the index of era
    */
    function payoutStakers(address vilidator, uint256[] memory era_index) external;


    /** @dev Try been validator
    * Selector:
    * @param payment_destination the index of enum RewardDestination
    */
    function setPayee(uint256 payment_destination) external;


    /** @dev Try been validator
    * Selector:
    * @param num_slashing_spans the determine the weight of the transaction (more storage items to remove means more weight, should get from SlashingSpans storage
    */
    function withdrawUnbonded(uint256 num_slashing_spans) external;


    /** @dev Try been validator
    * Selector:
    * @param validators the target validators want to nominate
    */
    function nominate(address[] memory validators) external;


    /** @dev Try been validator
    * Selector:
    * @param commission the proportion validator get
    * @param can_nominated if the validator want to be nominated
    */
    function validate(uint256 commission, bool can_nominated) external;

    /** @dev Set new session keys about the account
    * Selector:
    * @param session_keys the session key want to use
    * @param proof the proof about the session_keys
    */
    function setSessionKey(bytes memory session_keys, bytes memory proof) external;

    /** @dev Set new session keys and try to become validator
    * Selector:
    * @param commission the fee about commission
    * @param can_nominated if the validator want to be nominated
    * @param session_keys the session key want to use
    * @param proof the proof about the session_keys
    */
    function setSessionKeysAndValidate(uint256 commission, bool can_nominated, bytes memory session_keys, bytes memory proof) external;

    function chillAndUnbonded(uint256 value) external;

    function bondExtraAndNominate(uint256 bond_value, address[] memory validators) external;

    function bondExtraAndValidate(uint256 bond_value, uint256 commission, bool can_nominated, bytes memory session_keys, bytes memory proof) external;
}
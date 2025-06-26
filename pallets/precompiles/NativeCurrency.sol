// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.0;
/**
* @title Pallet AssetCurrency Interface
* @dev This interface does not exhaustively wrap pallet AssetCurrency, rather it wraps the most
* important parts and the parts that are expected to be most useful to evm contracts.
* More exhaustive wrapping can be added later if it is desireable and the pallet interface
* is deemed sufficiently stable.
* Address :  0x000000000000000000000000000000000000044e
*/

interface Native {
    function balanceOf(address account) external view returns (uint256);
    function mintTo(address to, uint256 amount) external;
    function burnFrom(address from, uint256 amount) external;
}

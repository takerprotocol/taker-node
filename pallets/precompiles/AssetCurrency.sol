// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.0;
/**
* @title Pallet AssetCurrency Interface
* @dev This interface does not exhaustively wrap pallet AssetCurrency, rather it wraps the most
* important parts and the parts that are expected to be most useful to evm contracts.
* More exhaustive wrapping can be added later if it is desireable and the pallet interface
* is deemed sufficiently stable.
* Address :  0x000000000000000000000000000000000000044d
*/

interface AssetCurrency {
    function balanceOf(address account) external view returns (uint256);
    function metadata() external view returns (string memory,uint256);
    function whitelistAdmin() external view returns (address);
    function whitelist() external view returns (address [] memory);
    function mintTo(address to, uint256 amount) external;
    function burn(address from, uint256 amount) external;
    function transferWhitelistAdmin(address admin) external;
    function updateWhitelist(address account, bool add) external;
    function transfer(address to, uint256 amount) external;
}
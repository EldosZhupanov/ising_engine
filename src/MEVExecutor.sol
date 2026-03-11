// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IERC20 {
    function balanceOf(address account) external view returns (uint256);
    function transfer(address to, uint256 amount) external returns (bool);
}

contract MEVExecutor {
    address public immutable owner;

    constructor() {
        owner = msg.sender;
    }

    modifier onlyOwner() {
        require(msg.sender == owner, "UNAUTHORIZED: Only Rust Bot");
        _;
    }

    function executeArbitrage(
        address[] calldata targets,
        bytes[] calldata payloads,
        address profitToken,
        uint256 minProfit
    ) external onlyOwner {
        uint256 balanceBefore = IERC20(profitToken).balanceOf(address(this));

        for (uint256 i = 0; i < targets.length; i++) {
            (bool success, bytes memory returnData) = targets[i].call(payloads[i]);
            if (!success) {
                revert(string(returnData));
            }
        }

        uint256 balanceAfter = IERC20(profitToken).balanceOf(address(this));
        require(balanceAfter >= balanceBefore + minProfit, "MEV_FAILED: No Profit");
    }

    function withdraw(address token) external onlyOwner {
        uint256 bal = IERC20(token).balanceOf(address(this));
        IERC20(token).transfer(owner, bal);
    }

    receive() external payable {}
}

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract SimpleAMM {
    address public token0;
    address public token1;
    uint256 public reserve0;
    uint256 public reserve1;
    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;

    uint256 public constant FEE = 3; // 0.3% fee (3/1000)

    event Mint(address indexed sender, uint256 amount0, uint256 amount1);
    event Burn(address indexed sender, uint256 amount0, uint256 amount1, address indexed to);
    event Swap(address indexed sender, uint256 amount0In, uint256 amount1In, uint256 amount0Out, uint256 amount1Out, address indexed to);

    constructor(address _token0, address _token1) {
        token0 = _token0;
        token1 = _token1;
    }

    function _mint(address to, uint256 amount) internal {
        balanceOf[to] += amount;
        totalSupply += amount;
    }

    function _burn(address from, uint256 amount) internal {
        balanceOf[from] -= amount;
        totalSupply -= amount;
    }

    function _update(uint256 bal0, uint256 bal1) internal {
        reserve0 = bal0;
        reserve1 = bal1;
    }

    function addLiquidity(uint256 amount0, uint256 amount1) external returns (uint256 liquidity) {
        _safeTransferFrom(token0, msg.sender, address(this), amount0);
        _safeTransferFrom(token1, msg.sender, address(this), amount1);

        uint256 bal0 = IERC20(token0).balanceOf(address(this));
        uint256 bal1 = IERC20(token1).balanceOf(address(this));

        if (totalSupply == 0) {
            liquidity = _sqrt(amount0 * amount1);
        } else {
            liquidity = _min(amount0 * totalSupply / reserve0, amount1 * totalSupply / reserve1);
        }

        require(liquidity > 0, "AMM: insufficient liquidity minted");
        _mint(msg.sender, liquidity);
        _update(bal0, bal1);

        emit Mint(msg.sender, amount0, amount1);
    }

    function removeLiquidity(uint256 liquidity) external returns (uint256 amount0, uint256 amount1) {
        require(liquidity > 0, "AMM: zero liquidity");
        amount0 = liquidity * reserve0 / totalSupply;
        amount1 = liquidity * reserve1 / totalSupply;
        require(amount0 > 0 && amount1 > 0, "AMM: insufficient output");

        _burn(msg.sender, liquidity);
        _update(reserve0 - amount0, reserve1 - amount1);

        _safeTransfer(token0, msg.sender, amount0);
        _safeTransfer(token1, msg.sender, amount1);

        emit Burn(msg.sender, amount0, amount1, msg.sender);
    }

    function swap(uint256 amount0Out, uint256 amount1Out, address to) external {
        require(amount0Out > 0 || amount1Out > 0, "AMM: zero output");
        require(amount0Out < reserve0 && amount1Out < reserve1, "AMM: insufficient liquidity");

        uint256 bal0 = IERC20(token0).balanceOf(address(this));
        uint256 bal1 = IERC20(token1).balanceOf(address(this));

        if (amount0Out > 0) _safeTransfer(token0, to, amount0Out);
        if (amount1Out > 0) _safeTransfer(token1, to, amount1Out);

        uint256 bal0New = IERC20(token0).balanceOf(address(this));
        uint256 bal1New = IERC20(token1).balanceOf(address(this));

        uint256 amount0In = bal0New > bal0 - amount0Out ? bal0New - (bal0 - amount0Out) : 0;
        uint256 amount1In = bal1New > bal1 - amount1Out ? bal1New - (bal1 - amount1Out) : 0;

        require(amount0In > 0 || amount1In > 0, "AMM: zero input");

        uint256 bal0Adjusted = bal0New * 1000 - amount0In * FEE;
        uint256 bal1Adjusted = bal1New * 1000 - amount1In * FEE;
        require(bal0Adjusted * bal1Adjusted >= uint256(reserve0) * reserve1 * 1000**2, "AMM: k invariant");

        _update(bal0New, bal1New);
        emit Swap(msg.sender, amount0In, amount1In, amount0Out, amount1Out, to);
    }

    function getAmountOut(uint256 amountIn, uint256 reserveIn, uint256 reserveOut) public pure returns (uint256) {
        require(amountIn > 0, "AMM: zero input");
        uint256 amountInWithFee = amountIn * (1000 - FEE);
        uint256 numerator = amountInWithFee * reserveOut;
        uint256 denominator = reserveIn * 1000 + amountInWithFee;
        return numerator / denominator;
    }

    function _sqrt(uint256 y) internal pure returns (uint256 z) {
        if (y > 3) {
            z = y;
            uint256 x = y / 2 + 1;
            while (x < z) { z = x; x = (y / x + x) / 2; }
        } else if (y != 0) { z = 1; }
    }

    function _min(uint256 a, uint256 b) internal pure returns (uint256) { return a < b ? a : b; }
    function _safeTransfer(address token, address to, uint256 value) internal {
        (bool success, bytes memory data) = token.call(abi.encodeWithSignature("transfer(address,uint256)", to, value));
        require(success && (data.length == 0 || abi.decode(data, (bool))), "AMM: transfer failed");
    }
    function _safeTransferFrom(address token, address from, address to, uint256 value) internal {
        (bool success, bytes memory data) = token.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", from, to, value));
        require(success && (data.length == 0 || abi.decode(data, (bool))), "AMM: transferFrom failed");
    }
}

interface IERC20 {
    function balanceOf(address account) external view returns (uint256);
    function transfer(address to, uint256 value) external returns (bool);
    function transferFrom(address from, address to, uint256 value) external returns (bool);
}

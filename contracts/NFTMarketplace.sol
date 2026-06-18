// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract NFTMarketplace {
    struct Listing {
        address seller;
        address nftContract;
        uint256 tokenId;
        uint256 price;
        uint256 royaltyPercentage;
        address royaltyRecipient;
        bool active;
    }

    uint256 public listingCount;
    uint256 public platformFee = 25; // 2.5% (25/1000)
    address public platformFeeRecipient;

    mapping(uint256 => Listing) public listings;
    mapping(address => mapping(uint256 => uint256)) public nftToListingId;

    event ItemListed(uint256 indexed listingId, address indexed seller, address indexed nftContract, uint256 tokenId, uint256 price);
    event ItemSold(uint256 indexed listingId, address indexed buyer, address indexed nftContract, uint256 tokenId, uint256 price);
    event ListingCanceled(uint256 indexed listingId);
    event ListingUpdated(uint256 indexed listingId, uint256 newPrice);

    constructor(address _platformFeeRecipient) {
        platformFeeRecipient = _platformFeeRecipient;
    }

    function listItem(address nftContract, uint256 tokenId, uint256 price, uint256 royaltyPercentage, address royaltyRecipient) external {
        require(price > 0, "Marketplace: price must be > 0");
        require(royaltyPercentage <= 100, "Marketplace: royalty too high");

        IERC721(nftContract).transferFrom(msg.sender, address(this), tokenId);

        listingCount++;
        listings[listingCount] = Listing(msg.sender, nftContract, tokenId, price, royaltyPercentage, royaltyRecipient, true);
        nftToListingId[nftContract][tokenId] = listingCount;

        emit ItemListed(listingCount, msg.sender, nftContract, tokenId, price);
    }

    function buyItem(uint256 listingId) external payable {
        Listing storage listing = listings[listingId];
        require(listing.active, "Marketplace: not active");
        require(msg.value == listing.price, "Marketplace: incorrect price");

        listing.active = false;

        uint256 platformCut = msg.value * platformFee / 1000;
        uint256 royaltyCut = listing.royaltyPercentage > 0 ? msg.value * listing.royaltyPercentage / 100 : 0;
        uint256 sellerProceeds = msg.value - platformCut - royaltyCut;

        if (royaltyCut > 0) {
            payable(listing.royaltyRecipient).transfer(royaltyCut);
        }
        payable(platformFeeRecipient).transfer(platformCut);
        payable(listing.seller).transfer(sellerProceeds);

        IERC721(listing.nftContract).transferFrom(address(this), msg.sender, listing.tokenId);

        emit ItemSold(listingId, msg.sender, listing.nftContract, listing.tokenId, listing.price);
    }

    function cancelListing(uint256 listingId) external {
        Listing storage listing = listings[listingId];
        require(listing.active, "Marketplace: not active");
        require(msg.sender == listing.seller, "Marketplace: not seller");

        listing.active = false;
        IERC721(listing.nftContract).transferFrom(address(this), listing.seller, listing.tokenId);

        emit ListingCanceled(listingId);
    }

    function updateListing(uint256 listingId, uint256 newPrice) external {
        Listing storage listing = listings[listingId];
        require(listing.active, "Marketplace: not active");
        require(msg.sender == listing.seller, "Marketplace: not seller");
        require(newPrice > 0, "Marketplace: price must be > 0");

        listing.price = newPrice;
        emit ListingUpdated(listingId, newPrice);
    }

    function updatePlatformFee(uint256 newFee) external {
        require(msg.sender == platformFeeRecipient, "Marketplace: unauthorized");
        require(newFee <= 100, "Marketplace: fee too high");
        platformFee = newFee;
    }
}

interface IERC721 {
    function transferFrom(address from, address to, uint256 tokenId) external;
    function safeTransferFrom(address from, address to, uint256 tokenId) external;
}

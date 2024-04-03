# ICE-randomizer
Random mint for ICE RRC404v1

#### How it works
1. People use `deposit()` to deposit WATER tokens and get NFT tickets.
2. Once enough tokens is deposited (at least 111 atm), the Owner calls `mint(n)`.   
This needs to be done in batches of n <= 40 via a script.
3. It will in turn call the ICE RRC404v1 component and mint a batch of ICE.  
Each ICE gets randomly assigned to someone holding an unused ticket.  
4. At any time, ticket owners can exchange their tickets to `withdraw()` either deposited WATER (unused tickets) or the randomly assigned ICE (used tickets).
5. Randomness is provided by [.Random](https://random-docs.radix.live/), ran by @Mleekko (what a coincidence!).
# ref-farming

## Terminology

|word|meaning|notes|
|-|-|-|
|Farm|A place to farming on|A farm has one seed and one reward|
|Seed|Farming-Token|User stakes seed to this contract to gain various reward token from all running farms using this seed|
|Reward|Reward-Token|A standard NEP-141 token that deposited to some farm as reward to farmers|
|SeedId|String|Token contract_id for ft token, token contract_id + "@" + inner_id for mft token|
|FarmId|String|SeedId + "#" + farm_index in that seed|
|RPS|Reward-Per-Seed|The key concept to distribute rewards between farmers in a farm, see next chapter for details|
|RR|Reward Round in num|The rewards are released by round, and round starts from 1, see next chapter for details|

## features
* Multi-farming, staking one seed token can gain multiple rewards on multiple farm;
* Farm creation is free and open;
* Both FT and multi-FT are supported as seed;


## Logic

### Basic Concept
**Farmer** deposit/stake **Seed** token to farming on all **farms** that accept that seed, and gains **reward** token back.   

different farm can (not must) has different reward token.  

### Farm Creation
To create a farm, we should settle five params:
* seed, which seed token this farm accepts;
* reward, which reward token this farm gives;
* when to start, in block height, if 0 is given, then imediately starts when reward token deposits in;
* round interval, each round lasts in block counts;  
* total reward per round;

And then transfer reward token into the farm using ft_transfer_call interface to bring farm_id information in.

### Reward Distribution
Every running farm would release reward per reward round. Those released reward would exist in a form of `user_unclaimed_reward` for each user.  

User has to explicit invoke `claim` action to fetch those reward into his inner account, and invoke `withdraw` to get those assets in inner account into his own near account. Although there are two txs here, the frontend could combine those actions to make it look like a one stop action.

Deep into reward distribution, it is caculated independently among farms and according to this logic:  

`user_unclaimed_reward = user_staked_seed * [RPS(farm) - RPS(user)]`  
where,   
`RPS(farm) = prev_RPS(farm) + to_be_distribute_reward / total_staked_seed`  
and,  
`RPS(user) = RPS(farm)` after user claims reward each time.

To figure out how many reward are waiting for distribution, we use RR to record the last distribution time. Then it turns to:  
`to_be_distribute_reward = reward_per_round * [RR(current) - RR(last)]`

the RR, RPS(farm) and RPS(user) will be updated after following actions are invoked:
* user claim reward;
* user stake seed (imbed a user claim process);
* user unstake seed(imbed a user claim process);

Note: to get detailed implement of that distribution logic, please refer to the contract readme.

### Farm status

* Created, A farm that has been created but either no reward token is deposited in or start time is not reached;
* Running, A farm that is in working and release reward per round;
* Ended, A farm with all reward has been distributed (user may still have unclaimed reward);
* Cleared, A farm that has ended and no unclaimed reward, can be removed from the contract. After removal, this farm is in this Cleared status. (You can never get this status from contract);


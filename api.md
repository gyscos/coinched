FORMAT: 1A

# coinched

The coinched API allows to communicate with a coinched server to play the coinche card game.

Desired properties:

* Calling twice the same page should not cause error
* Ideally, calling twice the same web page should return the same thing

Player refinement:

* Anonymous players, UUID generated on /join
* Named player, name is chosen on /join, no registration
* Registrated players, use password?

# Group Public
These methods can be called without a player ID.

## GET /help
Returns an help message with the available API endpoints.

+ Response 404 (applicatiion/json)

## POST /join
Attempt to join a new game. Will block until a party is found.

+ Response 200 (application/json)

        {
          "player_id": 123456,
          "player_pos": 2
        }

# Group General
These methods require a Player ID. Use `/join` to get one.

## GET /wait/{playerId}/{eventId}
Wait for the next event.

+ Response 200 (application/json)

        {
          "id": 1,
          "event": 0
        }

## POST /leave/{playerId}
Leave the game. The playerID becomes invalid after this call.

+ Response 200 (application/json)

        "ok"

## GET /hand/{playerId}
Returns the cards in hand for the given player, as a 32-bitset.

+ Response 200 (application/json)

        3

## GET /scores/{playerId}
Returns the scores for both teams.

+ Response 200 (application/json)

        [0, 0]

# Group Auction
These methods require a Player ID. They are only available during auction.

## POST /pass/{playerId}
Pass one's turn during auction.

+ Response 200 (application/json)

        {
          "id": 0,
          "event": {
            "type": "FromPlayer",
            "pos": 1,
            "event": {
              "type": "Passed"
            }
          }
        }


## POST /coinche/{playerId}
Coinche (or sur-coinche) the current contract.

+ Response 200 (application/json)

        {
          "id": 1,
          "event": {
            "type": "FromPlayer",
            "pos": 1,
            "event": {
              "type": "Coinched"
            }
          }
        }

# Group Game
These methods require a Player ID. They are only available during card play, after auction.

## GET /trick/{playerId}
Returns the current trick.

+ Response 200 (application/json)

        {
          "first": 1,
          "winner": 2,
          "cards": [ "None", "None", "None", "None" ]
        }

## GET /last_trick/{playerId}
Returns the last complete trick.

+ Response 200 (application/json)


        {
          "first": 1,
          "winner": 2,
          "cards": [ "None", "None", "None", "None" ]
        }

## POST /bid/{playerId}
+ Request (application/json)

        {
          "target": "80",
          "suit": 1
        }

+ Response 200 (application/json)

        {
          "id": 1,
          "event": {
            "type": "FromPlayer",
            "pos": 1,
            "event": {
              "type": "Bidded",
              "target": "80",
              "suit": 1
            }
          }
        }

## POST /play/{playerId}
+ Request (application/json)

        {
          "card": 64
        }

+ Response 200 (application/json)

        {
          "id": 2,
          "event": {
            "type": "FromPlayer",
            "pos": 1,
            "event": {
              "type": "CardPlayed",
              "card": 64
            }
          }
        }

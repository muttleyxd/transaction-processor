# Transaction processor

This in an exercise implementation of transaction processor.

## Brief

Transactions are defined as entries in CSV file
```
type,client,tx,amount
deposit,123,1,50.0
deposit,123,1,999.0
withdrawal,123,2,950.0
dispute,123,2
resolve,123,2
dispute,123,1
chargeback,123,1
```

Transactions describe operations done by clients to their account.
Data of each client contains:
- available funds
- held funds
- total funds
- information if account is locked (in that case no further operations are allowed)

Following operations are available:
- deposit: add amount to available funds of given client
- withdrawal: remove amount from available funds of given client, do nothing if there's not enough available funds
- dispute: start transaction reverse process, funds are moved to held
- resolve: successfully finish transaction revert process, transaction is reverted
- chargeback: forcefully revert transaction, account will get locked in result

Program processes these transactions and outputs information about clients and their data.
Output to above CSV file would be:
```
client,available,held,total,locked
123,50,0,50,true
```

## Usage

Following command will run the program with `example.csv` file:
```
cargo run -- example.csv
```

Logging errors to stderr can be enabled by using `-l` option:
```
cargo run -- -l example.csv
```

## Interesting bits

- Errors are handled silently by default, there is an option to enable them by using `-l` parameter.
Error logging is done into `stderr` stream, this means that errors are not affecting the output of the program.
I used `thiserror` crate for error handling, this allows to create a nice and human-readable description of error without losing information about types.
This design allows to have really detailed error handling, in case it's needed.

- Testing is mostly done through unit tests of `account_manager::account::Account`.
It contains tests for all possible operations, their possible combinations and error handling.
There's also an `example.csv` file which was used as a "system" test of whole program to ensure correct output format is being used.

- Performance of this program is limited by lack of async+multithreading support, this could possibly be much faster.
This program streams records from given file, so RAM usage does not increase linearly with file size.
However there is a limitation - client data is stored in a HashMap, so if there are too many clients and transactions, then we'll go out-of-memory.
Also we could do a little optimization by serializing directly from Accounts, instead of gathering all of the data to `Vec<OutputRecord>` before that, but that design looks little bit cleaner to me.

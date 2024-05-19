# Dropnote
Dropnote is a zero-knowledge, e2e, asymmetrically encrypted on-chain messaging system. This is its smart contract. [Here is its Dapp.](https://github.com/Kiruse/droptnote-frontend)

Recipients can deposit a public key for encryption on-chain. Senders can then store encrypted messages, likewise on-chain. Only the owner of the corresponding private key can decrypt and read these messages.

Messages do not have to be encrypted. This is useful to leave a dropnote for a wallet which hasn't registered a public encryption key yet. Obviously, such messages should not contain sensitive content.

Since Dropnote stores every single message on-chain, it is not adequate for real-time communication. Instead, it is intended to initiate communication with the owner of a wallet without knowing that wallet's contact details. This is, of course, still in no way a guarantee to actually reach the wallet owner.

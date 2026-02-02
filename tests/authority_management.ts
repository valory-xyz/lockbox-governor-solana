import * as anchor from "@coral-xyz/anchor";
import {
  getOrCreateAssociatedTokenAccount, syncNative, setAuthority, AuthorityType,
  createSetAuthorityInstruction, createTransferInstruction
} from "@solana/spl-token";
import { DecimalUtil, Percentage } from "@orca-so/common-sdk";
import bs58 from "bs58";

// UNIX/Linux/Mac
// bash$ export ANCHOR_PROVIDER_URL=http://127.0.0.1:8899
// bash$ export ANCHOR_WALLET=artifacts/id.json

async function main() {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const sol = new anchor.web3.PublicKey("So11111111111111111111111111111111111111112");
  const olas = new anchor.web3.PublicKey("Ez3nzG9ofodYCvEmw73XhQ87LWNYVRM2s7diB5tBZPyM");
  const multisigVault = new anchor.web3.PublicKey("GhGKWLW5yNqkwGJmGXg5ERhjveF57rHe4hwFCxVL3TeD");
  const source = new anchor.web3.PublicKey("Gn7oD4PmQth4ehA4b8PpHzq5v1UXPL61jAZd6CSuPvFU");
  const destination = new anchor.web3.PublicKey("H3ewsAuG9K2VaEL1abN4ZrearshpcQy5n5kDnt3HH23g");

  // User wallet is the provider payer
  const userWallet = provider.wallet["payer"];
  console.log("User wallet:", userWallet.publicKey.toBase58());

  let tokenOwnerAccountA;
    // Get the tokenA ATA of the userWallet address, and if it does not exist, create it
    tokenOwnerAccountA = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        userWallet,
        sol,
        userWallet.publicKey
    );
    console.log("User ATA for tokenA:", tokenOwnerAccountA.address.toBase58());

    let accountInfo = await provider.connection.getAccountInfo(tokenOwnerAccountA.address);
    //console.log(accountInfo);

    // Simulate SOL transfer and the sync of native SOL
    await provider.connection.requestAirdrop(tokenOwnerAccountA.address, 5000000000);
    await syncNative(provider.connection, userWallet, tokenOwnerAccountA.address);
    return;

    // !!!!!!!!!!!!!!!!! Proceed with caution after this line !!!!!!!!!!!!!!!!!

    // Update the token authority with the multisig vault address
    try {
      // Attempt to change owner of Associated Token Account
      await setAuthority(
        provider.connection, // Connection to use
        userWallet, // Payer of the transaction fee
        tokenOwnerAccountA.address, // Associated Token Account
        userWallet.publicKey, // Owner of the Associated Token Account
        AuthorityType.AccountOwner, // Type of Authority
        multisigVault
      );
    } catch (error) {
      console.log("\nExpect Error:", error);
    }

  // Transfer token instruction
  const transferInstruction = await createTransferInstruction(source, destination, multisigVault, 5000);
  console.log(transferInstruction);
  console.log(bs58.encode(transferInstruction.data));

  // SetAuthority instruction for the multisig to return back to the initial owner
  const setAuthorityInstruction = await createSetAuthorityInstruction(source, multisigVault,
    AuthorityType.AccountOwner, userWallet.publicKey);
  console.log(setAuthorityInstruction);
  console.log(bs58.encode(setAuthorityInstruction.data));
  //console.log(Buffer.from(setAuthorityInstruction.data).toString("base64"));
}

main();

import * as idl from "../target/idl/lockbox_governor.json";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  createMint, mintTo, transfer, getOrCreateAssociatedTokenAccount, syncNative, createAssociatedTokenAccount,
  unpackAccount, TOKEN_PROGRAM_ID, AccountLayout, getAssociatedTokenAddress, setAuthority, AuthorityType
} from "@solana/spl-token";
import expect from "expect";
import fs from "fs";
import bs58 from "bs58";

// UNIX/Linux/Mac
// bash$ export ANCHOR_PROVIDER_URL=http://127.0.0.1:8899
// bash$ export ANCHOR_WALLET=artifacts/id.json

async function main() {

  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const PROGRAM_ID = new anchor.web3.PublicKey("DWDGo2UkBUFZ3VitBfWRBMvRnHr7E2DSh57NK27xMYaB");
  const program = new Program(idl as anchor.Idl, PROGRAM_ID, anchor.getProvider());

  const chainId = 10002;
  const sequence = 7;
  const sol = new anchor.web3.PublicKey("So11111111111111111111111111111111111111112");
  const olas = new anchor.web3.PublicKey("Ez3nzG9ofodYCvEmw73XhQ87LWNYVRM2s7diB5tBZPyM");
  const wormhole = new anchor.web3.PublicKey("3u8hJUVTA4jH1wYAyUur7FFZVQ8H635K3tSHHF4ssjQ5");
  const posted = new anchor.web3.PublicKey("AdKqXRW51SyZgepKMs2x77kNYMv4CQfsjD7vResES9EQ");

  // This corresponds to Sepolia timelock address 000000000000000000000000471b3f60f08c50dd0ecba1bcd113b66fcc02b63d or 0x471b3f60f08c50dd0ecba1bcd113b66fcc02b63d
  const timelockBuffer = Buffer.from([
      0,   0,  0,   0,   0,   0,   0,   0,   0,
      0,   0,  0,  71,  27,  63,  96, 240, 140,
     80, 221, 14, 203, 161, 188, 209,  19, 182,
    111, 204,  2, 182,  61
  ]);
  const timelock = new anchor.web3.PublicKey(timelockBuffer);

  const vaaHashTransfer = Buffer.from([
    229,  55, 161, 184, 116, 103, 140,  23,
    219, 127, 125, 192,   9, 145, 174, 214,
    251,  24,  99, 144,  86,  16,  61,  17,
     26, 115,  37,  73, 254,  42, 223, 238
  ]);

  // User wallet is the provider payer
  const userWallet = provider.wallet["payer"];
  console.log("User wallet:", userWallet.publicKey.toBase58());

  console.log("timelock", timelock.toBase58());

    // Find a PDA account for the lockbox governor program
    const [pdaConfig, bumpConfig] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("config", "utf-8")],
        program.programId);
    //let bumpBytes = Buffer.from(new Uint8Array([bumpConfig]));
    console.log("Lockbox Governor PDA address:", pdaConfig.toBase58());
    console.log("Lockbox Governor PDA bump:", bumpConfig);

    let accountInfo = await provider.connection.getAccountInfo(olas);
    //console.log(accountInfo);

    // Get the tokenA ATA of the userWallet address, and if it does not exist, create it
    const tokenOwnerAccountA = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        userWallet,
        sol,
        userWallet.publicKey
    );
    console.log("User ATA for tokenA:", tokenOwnerAccountA.address.toBase58());
    //console.log("SOL ATA in hex", bs58.decode(tokenOwnerAccountA.address.toBase58()).toString("hex"));

    // Simulate SOL transfer and the sync of native SOL
    await provider.connection.requestAirdrop(tokenOwnerAccountA.address, 100000000000);
    await syncNative(provider.connection, userWallet, tokenOwnerAccountA.address);

    // Get the tokenA PDA ATA of the program Id, and if it does not exist, create it
    const pdaTokenAccountA = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        userWallet,
        sol,
        pdaConfig,
        true
    );
    console.log("Program PDA ATA for tokenA:", pdaTokenAccountA.address.toBase58());
    //console.log("SOL PDA ATA in hex", bs58.decode(pdaTokenAccountA.address.toBase58()).toString("hex"));

    // Simulate SOL transfer and the sync of native SOL
    await provider.connection.requestAirdrop(pdaTokenAccountA.address, 100000000000);
    await syncNative(provider.connection, userWallet, pdaTokenAccountA.address);

    // Sleep to wait for airdrops finalization
    await new Promise(f => setTimeout(f, 2000));

    // Get the tokenB ATA of the userWallet address, and if it does not exist, create it
    const tokenOwnerAccountB = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        userWallet,
        olas,
        userWallet.publicKey
    );
    console.log("User ATA for tokenB:", tokenOwnerAccountB.address.toBase58());

    // Get the tokenB PDA ATA of the userWallet address, and if it does not exist, create it
    const pdaTokenAccountB = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        userWallet,
        olas,
        pdaConfig,
        true
    );
    console.log("Program PDA ATA for tokenB:", pdaTokenAccountB.address.toBase58());

  let signature = null;

    // Initialize the LockboxGovernor program
    try {
        signature = await program.methods
          .initialize(chainId, timelockBuffer)
          .accounts(
            {
              config: pdaConfig,
            }
          )
          .rpc();
    } catch (error) {
        if (error instanceof Error && "message" in error) {
            console.error("Program Error:", error);
            console.error("Error Message:", error.message);
        } else {
            console.error("Transaction Error:", error);
        }
    }
    //console.log("Your transaction signature", signature);
    // Wait for program creation confirmation
    await provider.connection.confirmTransaction({
        signature: signature,
        ...(await provider.connection.getLatestBlockhash()),
    });

    console.log("Successfully initialized lockbox governor");

    accountInfo = await provider.connection.getAccountInfo(pdaConfig);
    //console.log(accountInfo);

    // Find a PDA account for the lockbox governor program
    let chainIdBuffer = Buffer.alloc(2);
    chainIdBuffer.writeUInt16LE(chainId, 0);
    let sequenceBuffer = Buffer.alloc(8);
    // NOTE! this needs to be adjusted with sequence number growing
    sequenceBuffer.writeUInt16LE(sequence, 0);
    const [pdaReceived, bumpReceived] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("received"),
        chainIdBuffer, sequenceBuffer], program.programId);
    //let bumpBytes = Buffer.from(new Uint8Array([bumpConfig]));
    console.log("Received PDA address:", pdaReceived.toBase58());
    console.log("Received PDA bump:", bumpReceived);

    // Receive message to transfer SOL funds
    try {
        signature = await program.methods
          .transfer(vaaHashTransfer)
          .accounts(
            {
              config: pdaConfig,
              wormholeProgram: wormhole,
              posted,
              received: pdaReceived,
              sourceAccount: pdaTokenAccountA.address,
              destinationAccount: tokenOwnerAccountA.address
            }
          )
          .rpc();
    } catch (error) {
        if (error instanceof Error && "message" in error) {
            console.error("Program Error:", error);
            console.error("Error Message:", error.message);
        } else {
            console.error("Transaction Error:", error);
        }
    }
    //console.log("Your transaction signature", signature);
    // Wait for program creation confirmation
    await provider.connection.confirmTransaction({
        signature: signature,
        ...(await provider.connection.getLatestBlockhash()),
    });

    console.log("Successfully transferred the funds");


    // Try to repeat the successful transaction
    try {
        signature = await program.methods
          .transfer(vaaHashTransfer)
          .accounts(
            {
              config: pdaConfig,
              wormholeProgram: wormhole,
              posted,
              received: pdaReceived,
              sourceAccount: pdaTokenAccountA.address,
              destinationAccount: tokenOwnerAccountA.address
            }
          )
          .rpc();
    } catch (error) {}
}

main();

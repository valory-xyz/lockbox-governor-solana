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
  // This corresponds to Sepolia timelock address 000000000000000000000000471b3f60f08c50dd0ecba1bcd113b66fcc02b63d or 0x471b3f60f08c50dd0ecba1bcd113b66fcc02b63d
  const timelockBuffer = Buffer.from([
      0,   0,  0,   0,   0,   0,   0,   0,   0,
      0,   0,  0,  71,  27,  63,  96, 240, 140,
     80, 221, 14, 203, 161, 188, 209,  19, 182,
    111, 204,  2, 182,  61
  ]);
  const timelock = new anchor.web3.PublicKey(timelockBuffer);

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

    let accountInfo = await provider.connection.getAccountInfo(pdaConfig);
    //console.log(accountInfo);

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
}

main();

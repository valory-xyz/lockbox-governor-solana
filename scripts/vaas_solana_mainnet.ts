import * as anchor from "@coral-xyz/anchor";
import { Transaction } from '@solana/web3.js';
import { encoding } from "@wormhole-foundation/sdk-base";
import { deserialize } from "@wormhole-foundation/sdk-definitions";
import { utils } from "@wormhole-foundation/sdk-solana-core";

// bash$ export ANCHOR_PROVIDER_URL=https://api.mainnet-beta.solana.com
// bash$ export ANCHOR_WALLET=wallet.json

async function main() {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    // User wallet is the provider payer
    const userWallet = provider.wallet["payer"];
    console.log("User wallet:", userWallet.publicKey.toBase58());

    const CHAIN_ID_POLYGON = 5;
    const emitter = "0x4cEB52802ef86edF8796632546d89e55c87a0901";
    const sequence = 0;
    const wormholeProgramId = new anchor.web3.PublicKey("worm2ZoG2kUd4vFXhvjh93UUH596ayRfgQ2MgjNMTth");
    const verifierSigner = new anchor.web3.PublicKey("CNWNtUbsGCTQGJsKAKWuns3rCTVtE2XGNV65yZdetVhu");

    // Also available under advanced view: https://wormholescan.io/#/tx/0x7b0145014a4e8f0d8621fbc0e366460dda3ba307732eff539f7c1e8e6589718a?view=advanced

    const vaaString = "AQAAAAQNAFm5t/rfQIt2blkom6awvaMiah4fWsbWAJ17Nw7oiM2TTmM6V4fcAKCnbn2lbKvgjh0B5glUrJ3fYFM/qvd9dhEAAet7QY4HCvqYI7YVZlyCk2wrPncH3pxbrVJFJXNB/k64F55VDPgK0NvYcW1ULW43XWdPDa8Bqp1oAJCwf81LQxQAAhqEQoBku8RmJpURq7ywOPbrdTCMfqYLuQ3eWvn2WbujNz9KW+TBzT2bslCmeoOOzdzGVbGiw4+1QdF+cIgbR+MABBhXLAUKDJq0TBaVQpqxLrm2KA7gEMhaNjIyJ0JTMr7MWUof+xuz7lHTgH0W/nhgNssoVjJ5TEozJe3DDP8l25EBBTLM7KLM6ev3Q1atcYtT/xW04+2aZI9gp5PqyBkZrErtRZ2nnTcEgZGV1RbtR3I1RMHVc5RYdocyq7IuDhuaRMwBCcetI7tC4yQLTudDkZDHkBkuLLbpO33l1fUKEHItaiJjXYdqh5ip2BPl43Ov8Q+cT2kwynZX+Oi+wZe6xZ0VPmMBCuIqCpcuHoNAOiIE2acoulU24FpsXuNSG+L6+D3dRpe8PA6YUBxrVm7erp4EqMFeFpKiFCUo2I0kqsE8KQsowIEAC/NFQmBOCRXLDlUpVg7AHwc5F8R42EgUhXDe0cS/0MH2ATUxraiNfojQ3BuVN+5su1FuHrZvqcFsCPums1tt++IBDHma7RPnEnDycxY2RQGq4Km6ewNVZKkfeX3aFbN/K5dfQJNgakhLY68M8yX8UgiQsoHPiJP1OldSe+RhhnBPrdoBDlQUF5aQBgxYw+30ry+sZU4hES2ooSSFCqxvyhvSDZN/WlXQw4bh4GA3YqzTgSeL119u6SviFhrV8K4qbeZVXUUBEKG/8at3+UTgiVAUlLpu/IsPFNdFWTY7uCQqNj/6+ujYY1dAYEMqRnmIwnEV1xXx4MJiq/G8SCoJPq4T7ku/oUEBEck144jvi8355MTwD50TOiNUQlhhTrBfmkt4mof9rY0mFU70oDh72pOnGtCioBY/TZbNZ7SoVE9TAZFa0PRKf/QBEkoxL9+BbdswqoGrCPdHTEFp5eAoOOWfMUe3xQuD0F9kTbU0XZ1Bb+Ea6uv8MbxALnD4rvvCHV3y1WCfFAEM3r8AZmCpvgAAAAAABQAAAAAAAAAAAAAAAEzrUoAu+G7fh5ZjJUbYnlXIegkBAAAAAAAAAAAAaGVsbG8gd29ybGQ=";

    const vaaBytes = encoding.b64.decode(vaaString);

    const parsed = deserialize("Uint8Array", vaaBytes);
    //console.log(parsed);

    //const [pdaProgram, bump] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("liquidity_lockbox", "utf-8")], program.programId);
    const [postedPDA, bump] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("PostedVAA"), parsed.hash], wormholeProgramId);
    console.log("Posted PDA address:", postedPDA.toBase58()); // Bq8zzYk9sAxFpaWUCUsZz16XgrD27QMNhPYR9wkirh5f
    console.log("Posted PDA bump:", bump);

    // Source: https://github.com/wormhole-foundation/wormhole-sdk-ts/blob/main/platforms/solana/protocols/core/src/core.ts
    // Somehow the script throws error on a signature, use the following repository to do the same in a one liner:
    // https://github.com/valory-xyz/wormhole-sdk-vaa.git
    const signatureSet = new anchor.web3.generate();

    // Get signature instructions
    const verifySignaturesInstructions =
        await utils.createVerifySignaturesInstructions(
            provider.connection,
            wormholeProgramId,
            userWallet.publicKey,
            parsed,
            signatureSet.publicKey,
        );

    // Create a new transaction for every 2 instructions
    for (let i = 0; i < verifySignaturesInstructions.length; i += 2) {
      const transaction = new Transaction().add(
        ...verifySignaturesInstructions.slice(i, i + 2),
      );
      transaction.feePayer = userWallet.publicKey;

      const blockHash = await provider.connection.getLatestBlockhash();
      transaction.recentBlockhash = blockHash.blockhash;

      const signature = await anchor.web3.sendAndConfirmTransaction(
        provider.connection,
        transaction,
        [signatureSet],
      );
      console.log("signature", signature);

        await provider.connection.confirmTransaction({
            signature: signature,
            ...(await provider.connection.getLatestBlockhash()),
        });
    }

    // Finally create the VAA posting transaction
    const transaction = new Transaction().add(
      utils.createPostVaaInstruction(
        provider.connection,
        wormholeProgramId,
        userWallet.publicKey,
        parsed,
        signatureSet.publicKey,
      ),
    );

    transaction.feePayer = userWallet.publicKey;

    const blockHash = await provider.connection.getLatestBlockhash();
    transaction.recentBlockhash = blockHash.blockhash;

    const signature = await anchor.web3.sendAndConfirmTransaction(
        provider.connection,
        transaction,
        [signatureSet],
    );
    console.log("signature", signature);
}

main();

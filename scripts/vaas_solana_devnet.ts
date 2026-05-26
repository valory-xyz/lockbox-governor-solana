import * as anchor from "@coral-xyz/anchor";
import { wormhole } from "@wormhole-foundation/sdk";
import solana from "@wormhole-foundation/sdk/solana";
import { encoding } from "@wormhole-foundation/sdk-base";
//import "../src/protocols/circleBridge/automaticCircleBridgeLayout.js";
import { deserialize, deserializePayload } from "@wormhole-foundation/sdk-definitions";
import { utils } from "@wormhole-foundation/sdk-solana-core";
import { secp256k1 } from "@noble/curves/secp256k1";

// bash$ export ANCHOR_PROVIDER_URL=https://api.devnet.solana.com
// bash$ export ANCHOR_WALLET=wallet.json

async function main() {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    // User wallet is the provider payer
    const userWallet = provider.wallet["payer"];
    console.log("User wallet:", userWallet.publicKey.toBase58());

    const CHAIN_ID_SEPOLIA = 10002;
    const emitter = "0x471B3f60f08C50dd0eCba1bCd113B66FCC02b63d";
    const sequence = 0;
    const wormholeProgramId = new anchor.web3.PublicKey("Bridge1p5gheXUvJ6jGWGeCsgPKgnE3YgdGKRVCMY9o");

    // See the vaa at: https://api.testnet.wormholescan.io/api/v1/vaas/10002/000000000000000000000000471b3f60f08c50dd0ecba1bcd113b66fcc02b63d/0'
    // Also available under advanced view: https://wormholescan.io/#/tx/0xd97e028b89995b8cdf66a798beb1a4fe25b594284b7669aed3a17ce95b6fddb4?network=TESTNET&view=advanced

    const vaaString = "AQAAAAABABGKXGn5H5ex4/H9h/Ibfsl+K5UYxnnH3JFUGtq0RCPtQ4UFdpJejL7C2Nt83mTQ7jA6GK1mEaL0Bh+jYXoXSLYAZl9PJAAAAAAnEgAAAAAAAAAAAAAAAEcbP2DwjFDdDsuhvNETtm/MArY9AAAAAAAAAAAAaGVsbG8gd29ybGQ=";

    const vaaBytes = encoding.b64.decode(vaaString);

    const parsed = deserialize("Uint8Array", vaaBytes);
    //console.log(parsed);

    //const [pdaProgram, bump] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("liquidity_lockbox", "utf-8")], program.programId);
    const [postedPDA, bump] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("PostedVAA"), parsed.hash], wormholeProgramId);
    console.log("Posted PDA address:", postedPDA.toBase58());
    console.log("Posted PDA bump:", bump);

//    const postedVaaAddress = utils.derivePostedVaaKey(
//      wormholeProgramId,
//      Buffer.from(parsed.hash),
//    );
//    console.log(postedVaaAddress.toBase58());

    const signatureSet = anchor.web3.Keypair.generate();

    const verifySignaturesInstructions =
        await utils.createVerifySignaturesInstructions(
            provider.connection,
            wormholeProgramId,
            userWallet.publicKey,
            parsed,
            signatureSet.publicKey,
        );

//    // Create a new transaction for every 2 instructions
//    for (let i = 0; i < verifySignaturesInstructions.length; i += 2) {
//      const verifySigTx = new Transaction().add(
//        ...verifySignaturesInstructions.slice(i, i + 2),
//      );
//      verifySigTx.feePayer = senderAddr;
//      yield this.createUnsignedTx(
//        { transaction: verifySigTx, signers: [signatureSet] },
//        'Core.VerifySignature',
//        true,
//      );
//    }
//
//    // Finally create the VAA posting transaction
//    const postVaaTx = new Transaction().add(
//      createPostVaaInstruction(
//        this.connection,
//        this.coreBridge.programId,
//        senderAddr,
//        vaa,
//        signatureSet.publicKey,
//      ),
//    );
//    postVaaTx.feePayer = senderAddr;
//
//    yield this.createUnsignedTx({ transaction: postVaaTx }, 'Core.PostVAA');
}

main();

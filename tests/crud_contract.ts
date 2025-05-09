import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { CrudContract } from "../target/types/crud_contract";
import { assert } from "chai";

describe("crud_contract", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.CrudContract as Program<CrudContract>;
  let global_accountPDA: anchor.web3.PublicKey;

  beforeEach(async () => {
    // Derive the global_account PDA
    [global_accountPDA] = await anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("global")],
      program.programId
    );

    // Initialize the global account
    await program.methods
      .initialize()
      .accounts({
        signer: provider.wallet.publicKey,
        globalAccount: global_accountPDA,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
  });

  it("Initializes the global state", async () => {
    // Took out the global_account inside the derive account struct Initialize
    // Inside Initialize derive struct, we have :
    // create global account, system_program account and signer account

    // Fetch current global account
    const globalAccount = await program.account.globalState.fetch(
      global_accountPDA
    );

    // case : program admin
    // case : total_text_created = 0
    // asset (actual , expect)
    assert.strictEqual(
      globalAccount.admin.toBase58(),
      provider.wallet.publicKey.toBase58(),
      "Admin should be the one Initialize Program and Global State"
    );

    assert.strictEqual(
      globalAccount.totalTextCreated.toNumber(),
      0,
      "Total text created at Initialize time should be 0"
    );
  });

  it("Create Text account", async () => {
    // Make sure another user calling create text

    // Fetch initial global account to get initial state variables value
    const initialGlobalAccount = await program.account.globalState.fetch(
      global_accountPDA
    );
    const initialTextCreated = initialGlobalAccount.totalTextCreated.toNumber();

    // derive create account
    const [create_accountPDA] =
      await anchor.web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("text"),
          Buffer.from(
            new anchor.BN(initialTextCreated).toArrayLike(Buffer, "le", 8)
          ),
        ],
        program.programId
      );

    // Then, user attempt to create text
    const title = "The Great Gatsby";
    const content = "A Leonardo Dicarpio Film in 2012";

    /**
     * #[account(
        mut,
        seeds = [b"global"],
        bump = global_account.bump
    )]
    pub global_account: Account<'info, GlobalState>,

    In order to took out the global_account in CreateText part, we need seeds, and correct bump = globalaccount.bump to match the PDA

    When Anchor validates a PDA, it re-derives the address using the specified 
    seeds and the bump from the account data (global_account.bump).
     */
    const globalAccount = await program.account.globalState.fetch(
      global_accountPDA
    );

    // call instruction
    const textTx = await program.methods
      .createText(title, content)
      .accounts({
        globalAccount: global_accountPDA,
        signer: provider.wallet.publicKey,
        createAccount: create_accountPDA,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("Your transaction signature (CreateText)", textTx);

    // Then, we fetch and verify the data
    const updatedGlobalAccount = await program.account.globalState.fetch(
      global_accountPDA
    );

    const totalCreated = updatedGlobalAccount.totalTextCreated.toNumber();
    console.log("totalCreated", totalCreated);

    assert.strictEqual(
      updatedGlobalAccount.totalTextCreated.toNumber(),
      initialTextCreated + 1,
      "Total text created should increment by 1"
    );

    // How about text data ?
    const create_account = await program.account.text.fetch(create_accountPDA);
    assert.strictEqual(
      create_account.id.toNumber(),
      initialTextCreated,
      "ID should match initial total"
    );
    assert.strictEqual(
      create_account.owner.toBase58(),
      provider.wallet.publicKey.toBase58(),
      "Owner should be the signer"
    );
    assert.strictEqual(create_account.title, title, "Title should match input");
    assert.strictEqual(
      create_account.content,
      content,
      "Content should match input"
    );
    assert.isAbove(
      create_account.createdAt.toNumber(),
      0,
      "Created at should be set"
    );
    assert.strictEqual(
      create_account.updatedAt.toNumber(),
      create_account.createdAt.toNumber(),
      "Updated at should match created at initially"
    );
  });

  // update sections
  it.only("Update text account", async () => {
    // We need create a text first before update into it
    const initialGlobalAccount = await program.account.globalState.fetch(
      global_accountPDA
    );
    const initialTextCreated = initialGlobalAccount.totalTextCreated.toNumber();
    console.log("initialTextCreated", initialTextCreated);

    // derive create account out first
    const title = "Transformer";
    const content = "Robot battle movies";
    const [create_accountPDA] =
      await anchor.web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("text"),
          Buffer.from(
            new anchor.BN(initialTextCreated).toArrayLike(Buffer, "le", 8)
          ),
        ],
        program.programId
      );

    // then create
    await program.methods
      .createText(title, content)
      .accounts({
        globalAccount: global_accountPDA,
        signer: provider.wallet.publicKey,
        createAccount: create_accountPDA,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const globalAccount = await program.account.globalState.fetch(
      global_accountPDA
    );
    const totalCreated = globalAccount.totalTextCreated;
    console.log("totalCreated", totalCreated);

    // after create, update new title
    const afterGlobalAccount = await program.account.globalState.fetch(
      global_accountPDA
    );
    const afterTextCreated = afterGlobalAccount.totalTextCreated;

    // derive update account, basically,  update account here is just updated data inside created acount
    // and that create_account already exist, we just need to point to them
    // using seeds and bump point to correct create account in previous create text part
    // we just naming them separately to easy understanding

    // call update instruction
    const updateTitle = "Scar face";
    const updateContent = "Tony motana";

    await program.methods
      .update(new anchor.BN(0), updateTitle, updateContent)
      .accounts({
        updateAccount: create_accountPDA,
        signer: provider.wallet.publicKey,
      })
      .rpc();

    const updatedTextAccount = await program.account.text.fetch(
      create_accountPDA
    );
    assert.strictEqual(
      updatedTextAccount.id.toNumber(),
      0,
      "ID should remain unchanged"
    );
    assert.strictEqual(
      updatedTextAccount.owner.toBase58(),
      provider.wallet.publicKey.toBase58(),
      "Owner should remain unchanged"
    );
    assert.strictEqual(
      updatedTextAccount.title,
      updateTitle,
      "Title should be updated"
    );
    assert.strictEqual(
      updatedTextAccount.content,
      updateContent,
      "Content should be updated"
    );
  });
});

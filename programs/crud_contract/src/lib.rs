use anchor_lang::prelude::*;

declare_id!("coUnmi3oBUtwtd9fjeAvSsJssXh5A5xyPbhpewyzRVF");

#[program]
pub mod crud_contract {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let _initialize = &mut ctx.accounts.global_account;

        _initialize.admin = ctx.accounts.signer.key();
        _initialize.total_text_created = 0;
        _initialize.bump = ctx.bumps.global_account;
        Ok(())
    }

    pub fn create_text(ctx: Context<CreateText>, title: String, content: String) -> Result<()> {
        let global_account = &mut ctx.accounts.global_account;
        let create_account = &mut ctx.accounts.create_account;
        let clock = Clock::get()?;

        // Initialize text account
        create_account.id = global_account.total_text_created;
        create_account.owner = ctx.accounts.signer.key(); // Correct owner
        create_account.title = title;
        create_account.content = content;
        create_account.created_at = clock.unix_timestamp;
        create_account.updated_at = clock.unix_timestamp;
        create_account.bump = ctx.bumps.create_account; // Matches create_account

        // Increment total_text_created
        global_account.total_text_created += 1;

        Ok(())
    }

    pub fn read(ctx: Context<ReadText>, id: u64) -> Result<(Text)> {
        let read_account = &mut ctx.accounts.read_account;

        Ok(Text {
            id: read_account.id,
            owner: read_account.owner,
            title: read_account.title.clone(),
            content: read_account.content.clone(),
            created_at: read_account.created_at,
            updated_at: read_account.updated_at,
            bump: read_account.bump,
        })
    }

    pub fn update(
        _ctx: Context<UpdateText>,
        _id: u64,
        new_title: String,
        new_content: String,
    ) -> Result<()> {
        // In this function, we have id as function argument, we dont need to use it inside our
        // instruction function, we inject it to our UpdateText context function, help us to form
        // the PDAs address
        let update_account = &mut _ctx.accounts.update_account;

        update_account.title = new_title;
        update_account.content = new_content;
        update_account.updated_at = Clock::get()?.unix_timestamp;

        Ok(())
    }

    pub fn delete(_ctx: Context<DeleteText>, _id: u64) -> Result<()> {
        let _signer = _ctx.accounts.signer.key();
        let delete_account = &mut _ctx.accounts.delete_account; // use mut because we delete
        let _admin = _ctx.accounts.global_account.admin; // not use because we using read-only field to check

        if delete_account.owner != _signer || _admin != _signer {
            return Err(ProgramError::InvalidAccountData.into());
        }

        _ctx.accounts.global_account.total_text_created -= 1;

        // Account is automatically closed by the `#[account(close = signer)]` attribute
        Ok(())
    }
}

// Initialize global state
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)] // Does it redundant ? Since we only call once ?
    // it tell Solana this signer account might be mutated during the instruction
    //The lamports balance can change. The data might change. The account might be closed or reassigned.
    pub signer: Signer<'info>, // who call, who charge
    #[account(
        init,
        payer = signer,
        space = 8 + GlobalState::INIT_SPACE,
        seeds= [b"global"],
        bump 
    )]
    pub global_account: Account<'info, GlobalState>,
    pub system_program: Program<'info, System>,
}

// How can we distinguish each create text account ?
// Seeds design : we are using two seeds : b"text" and global text created increment (dynamic)
// First text = seeds = [b"text", 0] → PDA_0
// Second text = seeds = [b"text", 1] → PDA_1
// Third text = seeds = [b"text", 2] → PDA_2
// This ensure : each created text have unique PDAs
// Let’s say total_text_created = 42.
// Then, our PDA becomes : seeds = [b"text", 42 as bytes]
// text.id = 42;

// If we want a user have a list of text created, we have another design for seeds
// We combine : user signer + global text count + b"text"
// So, what does b"text" in here ?
// It act as a domain prefix in PDA seeds, it like a table name in SQL
#[derive(Accounts)]
#[instruction(title: String, content: String)]
pub struct CreateText<'info> {
    /**
     * Purpose of re-declare global_account: provide total_text_created for the Text account's seeds
     * Why we re-declare seeds and bump here when we already initialized it ?
     * The reson is because security and validation
     * The address of global_state is derived from b"global" and the programID + bump to ensure it off-curve
     * When we include the global_state in an instruction (CreateText instruction). Anchor need to verify that
     * the provided global_state account is the correct PDA, prevent SO passing a fake account that mimics GlobalState
     * but has a different address
     *
     * Summary : Every instruction that use it must re-validate its PDA status
     * In Solana, anyone can call your program’s instructions with any accounts they choose,
     * unless you validate them. Without proper checks (like seeds and bump for PDAs),
     * a malicious user could pass a fake account that mimics your GlobalState struct but has
     * incorrect data (e.g., a wrong total_text_created). This could disrupt your program’s logic,
     * like creating Text accounts with duplicate or incorrect PDAs.
     * => Only the legitimate global_state PDA (created in Initialize) is accepted.
     *
     * Bonus : Real Usage of #[account(...)] Macro
     * With init (Creates a PDA or Account):
     * Without init (Checks an Existing Account):Purpose: Validates that an existing account
     * matches the expected properties (e.g., is a PDA, has correct data).PDA Validation: If seeds and bump are provided,
     *  it verifies the account is the correct PDA by matching its address to the derived PDA.
     */
    #[account(
        mut,
        seeds = [b"global"],
        bump = global_account.bump // check using seeds + bump
    )]
    pub global_account: Account<'info, GlobalState>,
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        space = 8 + Text::INIT_SPACE,
        payer = signer,
        seeds = [b"text", global_account.total_text_created.to_le_bytes().as_ref()], 
        bump,
    )]
    pub create_account: Account<'info, Text>,
    pub system_program: Program<'info, System>,
}

// Anchor/Solana programs can't return values like typical functions because state lives in account, not program(Contract)
/**
 * Reading data mean we accessing account data via their address in a client or instruction
 * We will need The accounts struct for the ReadText instruction and The instruction logic to read the Text account.
 * What the instruction will do ? It will derive the text account PDAs from ReadText derive account struct using the seeds
 * we access the Text account directly, validating its PDA to ensure it’s the correct account.
 *
 */
/**
 * What this derive struct will have ?
 * read_account : the text account to read from, we validate it with PDAs seeds [b"text", id.to_le_bytes()]
 * global_account : include it to validate account
 * #[instruction(id: u64)]: Allows the instruction to access the id parameter to derive the PDA seeds.
 * Since this is read-only, we don’t need mut, a signer or system program unless additional checks are required.
 * global_account is included for optional validation. It allows you to check if
 * the provided id is valid (e.g., id < global_account.total_text_created) to ensure the Text account exists.
 * For example :
 * if id >= ctx.accounts.global_account.total_text_created {
    return Err(ErrorCode::InvalidId.into());
    }
 *
 *
 */
#[derive(Accounts)]
#[instruction(id: u64)]
pub struct ReadText<'info> {
    #[account(
        mut,
        seeds=[b"global"],
        bump= global_account.bump
    )]
    pub global_account: Account<'info, GlobalState>, // optional, PDA for validation

    // does not need to init, just access existing data
    #[account(
        seeds=[b"text", id.to_le_bytes().as_ref()],
        bump=read_account.bump
    )]
    pub read_account: Account<'info, Text>, // PDA
}

// Update Section
// When we let user update text, we update into already initialized "create_account" account
// or we create a new account update_account ?
// In Create part : we have seeds = global_account.total_text_created incrementally
// It updated to already existing account

// Next question is : how the updated part find correctly the id of the text we created in the create section, how the id
// in the seeds find correct the total_text_created in the create account seeds part
// You pass in id: 42, so the PDA becomes: seeds = [b"text", 42 as bytes] // ← Same as line 65
// Solana + Anchor resolves that exact same PDA address that was created earlier
#[derive(Accounts)]
#[instruction(id: u64)]
pub struct UpdateText<'info> {
    #[account(mut)]
    pub signer: Signer<'info>, // who updated, who charge
    // #[account(
    //     seeds=[b"global"],
    //     bump = global_account.bump
    // )]
    // pub global_account: Account<'info, GlobalState>, // took this out for validate (id passed in > total_text_created), but turn out it does not neede

    // here, it reference to already existing account using exact same seeds we use into createText
    // We resolving it from the same seeds that we used to initialize within Create part
    // One more notes 🔥 : Since in the create part, that create account id incremented attach with that signer
    // So, we can easily understand because only that id belong to that signer, we dont need to care
    // about correctly updated to which user
    #[account(
        mut,
        seeds=[b"text", id.to_le_bytes().as_ref()], 
        bump = update_account.bump,
        constraint = update_account.owner == signer.key() @ ErrorCode::Unauthorized,
    )]
    pub update_account: Account<'info, Text>,
    // No System Program: No account creation, so it’s not required.
}

// Delete part
// In delete section, we can think as : both admin and the owner of the text can delete it
// Solana does not let us delete account, unless close them
#[derive(Accounts)]
#[instruction(id: u64)]
pub struct DeleteText<'info> {
    #[account(
        mut,
        seeds=[b"global"],
        bump = global_account.bump)]
    pub global_account: Account<'info, GlobalState>, // using it to decrement the text created
    #[account(mut)]
    pub signer: Signer<'info>, // Who call delete, who charge

    #[account(
        mut,
        seeds=[b"text", id.to_le_bytes().as_ref()], 
        bump = delete_account.bump,
        constraint = delete_account.owner == signer.key() @ ErrorCode::Unauthorized,
        close = signer, // lamports returned to whoever called
    )]
    pub delete_account: Account<'info, Text>,
}

#[account]
#[derive(InitSpace)]
pub struct GlobalState {
    pub admin: Pubkey,
    pub total_text_created: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Text {
    pub id: u64, // Help us to keep track
    pub owner: Pubkey,
    #[max_len(50)]
    pub title: String,
    #[max_len(1000)]
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized: Signer is not the owner")]
    Unauthorized,
}

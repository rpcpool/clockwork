use {
    crate::state::*,
    anchor_lang::{prelude::*, solana_program::{system_program, instruction::Instruction, sysvar}},
    std::mem::size_of,
};

const SIZE_OF_HEALTH_CHECK_IX: usize = 80;

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct AdminScheduleHealthCheck<'info> {
    #[account(mut, address = config.admin)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [SEED_AUTHORITY], 
        bump = authority.bump, 
        owner = crate::ID
    )]
    pub authority: Account<'info, Authority>,
    
    #[account(address = sysvar::clock::ID)]
    pub clock: Sysvar<'info, Clock>,

    #[account(
        seeds = [SEED_CONFIG],
        bump = config.bump,
        owner = crate::ID,
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [
            SEED_DAEMON, 
            daemon.owner.as_ref()
        ],
        bump = daemon.bump,
        constraint = daemon.owner == authority.key(),
        owner = crate::ID,
    )]
    pub daemon: Account<'info, Daemon>,

    #[account(
        mut,
        seeds = [SEED_HEALTH],
        bump = health.bump,
        owner = crate::ID,
    )]
    pub health: Account<'info, Health>,

    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,

    #[account(
        init,
        seeds = [
            SEED_TASK, 
            daemon.key().as_ref(),
            daemon.task_count.to_be_bytes().as_ref(),
        ],
        bump = bump,
        payer = admin,
        space = 32 + size_of::<Task>() + SIZE_OF_HEALTH_CHECK_IX, 
    )]
    pub task: Account<'info, Task>,
}

pub fn handler(ctx: Context<AdminScheduleHealthCheck>, bump: u8) -> ProgramResult {
    // Get accounts.
    let authority = &ctx.accounts.authority;
    let clock = &ctx.accounts.clock;
    let daemon = &mut ctx.accounts.daemon;
    let health = &mut ctx.accounts.health;
    let task = &mut ctx.accounts.task;

    // Setup the health account.
    let now = clock.unix_timestamp;
    let exec_at = now.checked_add(1).unwrap();
    health.real_time = now;
    health.target_time = exec_at;

    // Create health check instruction
    let health_check_ix = InstructionData::from(
        Instruction {
            program_id: crate::ID,
            accounts: vec![
                AccountMeta::new_readonly(clock.key(), false),
                AccountMeta::new_readonly(authority.key(), false),
                AccountMeta::new(daemon.key(), true),
                AccountMeta::new(health.key(), false),
            ],
            data: vec![],
        }
    );

    // Initialize task account.
    task.daemon = daemon.key();
    task.id = daemon.task_count;
    task.ix = health_check_ix;
    task.status = TaskStatus::Pending;
    task.exec_at = exec_at;
    task.stop_at = i64::MAX;
    task.recurr = 1;
    task.bump = bump;

    // Increment daemon task counter.
    daemon.task_count = daemon.task_count.checked_add(1).unwrap();

    Ok(())
}

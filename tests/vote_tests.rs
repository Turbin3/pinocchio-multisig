use pinocchio_multisig::{
    helper::StateDefinition,
    state::{ProposalState, ProposalType},
};
use {solana_keypair::Keypair, solana_signer::Signer};

mod common;

#[test]
pub fn test_first_vote_yes() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();

    let third_admin = Keypair::new();
    svm.airdrop(&third_admin.pubkey(), 100000000).unwrap();
    let admins = vec![second_admin.pubkey(), third_admin.pubkey()];

    let (pda_multisig, multisig_bump) =
        common::create_multisig(&mut svm, &fee_payer, program_id, admins);

    let (pda_proposal, proposal_bump) = common::create_proposal(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        ProposalType::Cpi,
    );

    // First vote: Yes
    common::vote(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        1,
    );

    let proposal_account = svm.get_account(&pda_proposal).unwrap();
    let proposal_data = &proposal_account.data;
    let proposal_state = ProposalState::from_bytes(proposal_data).unwrap();
    assert_eq!(proposal_state.yes_votes, 1);
    assert_eq!(proposal_state.no_votes, 0);

    let proposal_votes_data = &proposal_data[ProposalState::LEN..];
    let proposal_votes = proposal_votes_data.chunks_exact(32).collect::<Vec<&[u8]>>();
    assert_eq!(proposal_votes.len(), 1);
    assert_eq!(proposal_votes[0], second_admin.pubkey().as_ref());
}

#[test]
pub fn test_first_vote_no() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();

    let third_admin = Keypair::new();
    svm.airdrop(&third_admin.pubkey(), 100000000).unwrap();
    let admins = vec![second_admin.pubkey(), third_admin.pubkey()];
    let (pda_multisig, multisig_bump) =
        common::create_multisig(&mut svm, &fee_payer, program_id, admins);

    let (pda_proposal, proposal_bump) = common::create_proposal(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        ProposalType::Cpi,
    );

    // First vote: No
    common::vote(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        0,
    );

    let proposal_account = svm.get_account(&pda_proposal).unwrap();
    let proposal_data = &proposal_account.data;
    let proposal_state = ProposalState::from_bytes(proposal_data).unwrap();
    assert_eq!(proposal_state.yes_votes, 0);
    assert_eq!(proposal_state.no_votes, 1);

    let proposal_votes_data = &proposal_data[ProposalState::LEN..];
    let proposal_votes = proposal_votes_data.chunks_exact(32).collect::<Vec<&[u8]>>();
    assert_eq!(proposal_votes.len(), 1);
    assert_eq!(proposal_votes[0], second_admin.pubkey().as_ref());
}

#[test]
pub fn test_change_vote_from_yes_to_no() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();

    let third_admin = Keypair::new();
    svm.airdrop(&third_admin.pubkey(), 100000000).unwrap();
    let admins = vec![second_admin.pubkey(), third_admin.pubkey()];

    let (pda_multisig, multisig_bump) =
        common::create_multisig(&mut svm, &fee_payer, program_id, admins);

    let (pda_proposal, proposal_bump) = common::create_proposal(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        ProposalType::Cpi,
    );

    // First vote: Yes
    common::vote(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        1,
    );

    // Change vote: Yes to No
    common::vote(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        0,
    );

    let proposal_account = svm.get_account(&pda_proposal).unwrap();
    let proposal_data = &proposal_account.data;
    let proposal_state = ProposalState::from_bytes(proposal_data).unwrap();
    assert_eq!(proposal_state.yes_votes, 0);
    assert_eq!(proposal_state.no_votes, 1);

    let proposal_votes_data = &proposal_data[ProposalState::LEN..];
    let proposal_votes = proposal_votes_data.chunks_exact(32).collect::<Vec<&[u8]>>();
    assert_eq!(proposal_votes.len(), 1);
    assert_eq!(proposal_votes[0], second_admin.pubkey().as_ref());
}

#[test]
pub fn test_change_vote_from_no_to_yes() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();

    let third_admin = Keypair::new();
    svm.airdrop(&third_admin.pubkey(), 100000000).unwrap();
    let admins = vec![second_admin.pubkey(), third_admin.pubkey()];

    let (pda_multisig, multisig_bump) =
        common::create_multisig(&mut svm, &fee_payer, program_id, admins);

    let (pda_proposal, proposal_bump) = common::create_proposal(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        ProposalType::Cpi,
    );

    // First vote: No
    common::vote(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        0,
    );

    // Change vote: No to Yes
    common::vote(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        1,
    );

    let proposal_account = svm.get_account(&pda_proposal).unwrap();
    let proposal_data = &proposal_account.data;
    let proposal_state = ProposalState::from_bytes(proposal_data).unwrap();
    assert_eq!(proposal_state.yes_votes, 1);
    assert_eq!(proposal_state.no_votes, 0);

    let proposal_votes_data = &proposal_data[ProposalState::LEN..];
    let proposal_votes = proposal_votes_data.chunks_exact(32).collect::<Vec<&[u8]>>();
    assert_eq!(proposal_votes.len(), 1);
    assert_eq!(proposal_votes[0], second_admin.pubkey().as_ref());
}

#[test]
pub fn test_multiple_votes_yes_then_no() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();

    let third_admin = Keypair::new();
    svm.airdrop(&third_admin.pubkey(), 100000000).unwrap();
    let admins = vec![second_admin.pubkey(), third_admin.pubkey()];

    let (pda_multisig, multisig_bump) =
        common::create_multisig(&mut svm, &fee_payer, program_id, admins);

    let (pda_proposal, proposal_bump) = common::create_proposal(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        ProposalType::Cpi,
    );

    // Vote Yes multiple times (should change to No after first)
    common::vote(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        1,
    );
    println!("Voted Yes by second admin");

    common::vote(
        &mut svm,
        &third_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        1,
    );

    println!("Voted Yes by third admin");

    let proposal_account = svm.get_account(&pda_proposal).unwrap();
    let proposal_data = &proposal_account.data;
    let proposal_state = ProposalState::from_bytes(proposal_data).unwrap();
    assert_eq!(proposal_state.yes_votes, 2);
    assert_eq!(proposal_state.no_votes, 0);

    svm.warp_to_slot(1);

    // Try to vote Yes again (should change to No)
    common::vote(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        0,
    );

    let proposal_account = svm.get_account(&pda_proposal).unwrap();
    let proposal_data = &proposal_account.data;
    let proposal_state = ProposalState::from_bytes(proposal_data).unwrap();
    assert_eq!(proposal_state.yes_votes, 1);
    assert_eq!(proposal_state.no_votes, 1);

    let proposal_votes_data = &proposal_data[ProposalState::LEN..];
    let proposal_votes = proposal_votes_data.chunks_exact(32).collect::<Vec<&[u8]>>();
    assert_eq!(proposal_votes.len(), 2);
    assert_eq!(proposal_votes[0], third_admin.pubkey().as_ref());
    assert_eq!(proposal_votes[1], second_admin.pubkey().as_ref());
}

#[test]
pub fn test_multiple_votes_no_then_yes() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();

    let third_admin = Keypair::new();
    svm.airdrop(&third_admin.pubkey(), 100000000).unwrap();
    let admins = vec![second_admin.pubkey(), third_admin.pubkey()];
    let (pda_multisig, multisig_bump) =
        common::create_multisig(&mut svm, &fee_payer, program_id, admins);

    let (pda_proposal, proposal_bump) = common::create_proposal(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        ProposalType::Cpi,
    );

    // Vote No multiple times (should change to Yes after first)
    common::vote(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        0,
    );

    common::vote(
        &mut svm,
        &third_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        0,
    );

    // Try to vote No again (should change to Yes)
    common::vote(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        1,
    );

    let proposal_account = svm.get_account(&pda_proposal).unwrap();
    let proposal_data = &proposal_account.data;
    let proposal_state = ProposalState::from_bytes(proposal_data).unwrap();
    assert_eq!(proposal_state.yes_votes, 1);
    assert_eq!(proposal_state.no_votes, 1);

    let proposal_votes_data = &proposal_data[ProposalState::LEN..];
    let proposal_votes = proposal_votes_data.chunks_exact(32).collect::<Vec<&[u8]>>();
    assert_eq!(proposal_votes.len(), 2);
    assert_eq!(proposal_votes[1], third_admin.pubkey().as_ref());
    assert_eq!(proposal_votes[0], second_admin.pubkey().as_ref());
}

#[test]
pub fn test_vote_alternating_pattern() {
    let (mut svm, fee_payer, second_admin, program_id) = common::setup_svm_and_program();

    let third_admin = Keypair::new();
    svm.airdrop(&third_admin.pubkey(), 100000000).unwrap();
    let admins = vec![second_admin.pubkey(), third_admin.pubkey()];

    let (pda_multisig, multisig_bump) =
        common::create_multisig(&mut svm, &fee_payer, program_id, admins);

    let (pda_proposal, proposal_bump) = common::create_proposal(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        ProposalType::Cpi,
    );

    // Vote Yes
    common::vote(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        1,
    );

    let proposal_account = svm.get_account(&pda_proposal).unwrap();
    let proposal_data = &proposal_account.data;
    let proposal_state = ProposalState::from_bytes(proposal_data).unwrap();
    assert_eq!(proposal_state.yes_votes, 1);
    assert_eq!(proposal_state.no_votes, 0);

    // Change to No
    common::vote(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        0,
    );

    let proposal_account = svm.get_account(&pda_proposal).unwrap();
    let proposal_data = &proposal_account.data;
    let proposal_state = ProposalState::from_bytes(proposal_data).unwrap();
    assert_eq!(proposal_state.yes_votes, 0);
    assert_eq!(proposal_state.no_votes, 1);

    svm.expire_blockhash();

    // Change back to Yes
    common::vote(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        1,
    );

    let proposal_account = svm.get_account(&pda_proposal).unwrap();
    let proposal_data = &proposal_account.data;
    let proposal_state = ProposalState::from_bytes(proposal_data).unwrap();
    assert_eq!(proposal_state.yes_votes, 1);
    assert_eq!(proposal_state.no_votes, 0);

    // Change back to No
    common::vote(
        &mut svm,
        &second_admin,
        program_id,
        pda_multisig,
        multisig_bump,
        pda_proposal,
        proposal_bump,
        0,
    );

    let proposal_account = svm.get_account(&pda_proposal).unwrap();
    let proposal_data = &proposal_account.data;
    let proposal_state = ProposalState::from_bytes(proposal_data).unwrap();
    assert_eq!(proposal_state.yes_votes, 0);
    assert_eq!(proposal_state.no_votes, 1);

    let proposal_votes_data = &proposal_data[ProposalState::LEN..];
    let proposal_votes = proposal_votes_data.chunks_exact(32).collect::<Vec<&[u8]>>();
    assert_eq!(proposal_votes.len(), 1);
    assert_eq!(proposal_votes[0], second_admin.pubkey().as_ref());
}

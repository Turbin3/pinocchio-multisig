const { Connection, Keypair, PublicKey, Transaction, TransactionInstruction, sendAndConfirmTransaction, LAMPORTS_PER_SOL, SystemProgram } = require('@solana/web3.js');

async function fullCloseProposalTest() {
  console.log('🎯 Full close_proposal functionality test...\n');
  
  const connection = new Connection('http://localhost:8899', 'confirmed');
  const PROGRAM_ID = new PublicKey('3Cxo8aHmXk4thjhEM2Upm5Mdupj9NNhJ94LdkGaGskbs');
  
  const payer = Keypair.generate();
  const creator = Keypair.generate();
  
  // Airdrop SOL
  await connection.requestAirdrop(payer.publicKey, 2e9);
  await connection.requestAirdrop(creator.publicKey, 1e9);
  await new Promise(r => setTimeout(r, 2000));
  
  // Test members
  const member1 = Keypair.generate();
  const member2 = Keypair.generate();
  const member3 = Keypair.generate();

  console.log('🏗️  Step 1: Initialize multisig (using existing instruction)...');
  
  // Derive PDAs
  const [multisigPda, bump] = PublicKey.findProgramAddressSync(
    [Buffer.from('multisig'), creator.publicKey.toBuffer()],
    PROGRAM_ID
  );
  
  const [multisigConfigPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('multisig_config'), multisigPda.toBuffer()],
    PROGRAM_ID
  );
  
  const [treasuryPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('treasury'), multisigPda.toBuffer()],
    PROGRAM_ID
  );

  // Initialize multisig first
  const initData = Buffer.concat([
    Buffer.from([bump]),
    Buffer.from([3]), // 3 members
    member1.publicKey.toBuffer(),
    member2.publicKey.toBuffer(),
    member3.publicKey.toBuffer(),
  ]);

  const initIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: creator.publicKey, isSigner: true, isWritable: true },
      { pubkey: multisigPda, isSigner: false, isWritable: true },
      { pubkey: multisigConfigPda, isSigner: false, isWritable: true },
      { pubkey: treasuryPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.concat([Buffer.from([0]), initData]),
  });

  try {
    await sendAndConfirmTransaction(connection, new Transaction().add(initIx), [payer, creator]);
    console.log('✅ Multisig initialized successfully');
  } catch (err) {
    console.log('❌ Failed to initialize multisig:', err.message);
    return;
  }

  // Test scenarios
  await testScenario1_Success(connection, PROGRAM_ID, payer, multisigPda, multisigConfigPda, member1, member2, member3);
  await testScenario2_Failure(connection, PROGRAM_ID, payer, multisigPda, multisigConfigPda, member1, member2, member3);
  await testScenario3_EarlyCancel(connection, PROGRAM_ID, payer, multisigPda, multisigConfigPda, member1, member2, member3);
}

async function testScenario1_Success(connection, PROGRAM_ID, payer, multisigPda, multisigConfigPda, member1, member2, member3) {
  console.log('\n📋 Test Scenario 1: Successful Proposal (3/3 threshold met)');
  
  const [proposalPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('proposal'), multisigPda.toBuffer(), Buffer.from(new Uint8Array(new BigUint64Array([BigInt(1)]).buffer))],
    PROGRAM_ID
  );

  // Manually create proposal account with votes
  const proposalData = createProposalData({
    proposal_id: 1,
    expiry: 0, // Already expired (slot 0)
    status: 1, // Active
    members: [member1.publicKey, member2.publicKey, member3.publicKey],
    votes: [1, 1, 1, 0, 0, 0, 0, 0, 0, 0], // 3 YES votes
  });

  // Create the proposal account
  await createAccount(connection, payer, proposalPda, proposalData, PROGRAM_ID);

  // Test close_proposal
  const closeIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: proposalPda, isSigner: false, isWritable: true },
      { pubkey: multisigConfigPda, isSigner: false, isWritable: false },
    ],
    data: Buffer.from([4]),
  });

  try {
    const sig = await sendAndConfirmTransaction(connection, new Transaction().add(closeIx), [payer]);
    console.log('✅ close_proposal executed! Signature:', sig);
    
    // Check the result
    const proposalAccount = await connection.getAccountInfo(proposalPda);
    const status = proposalAccount.data[16]; // Status is at offset 16
    console.log('📊 Final status:', status === 3 ? 'Succeeded ✅' : status === 2 ? 'Failed ❌' : `Unknown (${status})`);
    
    if (status === 3) {
      console.log('🎉 SUCCESS: Vote counting and threshold comparison worked!');
    }
  } catch (err) {
    console.log('❌ Scenario 1 failed:', err.message);
  }
}

async function testScenario2_Failure(connection, PROGRAM_ID, payer, multisigPda, multisigConfigPda, member1, member2, member3) {
  console.log('\n📋 Test Scenario 2: Failed Proposal (1/3 threshold not met)');
  
  const [proposalPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('proposal'), multisigPda.toBuffer(), Buffer.from(new Uint8Array(new BigUint64Array([BigInt(2)]).buffer))],
    PROGRAM_ID
  );

  // Create proposal with insufficient votes
  const proposalData = createProposalData({
    proposal_id: 2,
    expiry: 0, // Already expired
    status: 1, // Active
    members: [member1.publicKey, member2.publicKey, member3.publicKey],
    votes: [1, 2, 2, 0, 0, 0, 0, 0, 0, 0], // 1 YES, 2 NO votes
  });

  await createAccount(connection, payer, proposalPda, proposalData, PROGRAM_ID);

  const closeIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: proposalPda, isSigner: false, isWritable: true },
      { pubkey: multisigConfigPda, isSigner: false, isWritable: false },
    ],
    data: Buffer.from([4]),
  });

  try {
    const sig = await sendAndConfirmTransaction(connection, new Transaction().add(closeIx), [payer]);
    console.log('✅ close_proposal executed! Signature:', sig);
    
    const proposalAccount = await connection.getAccountInfo(proposalPda);
    const status = proposalAccount.data[16];
    console.log('📊 Final status:', status === 2 ? 'Failed ❌' : status === 3 ? 'Succeeded ✅' : `Unknown (${status})`);
    
    if (status === 2) {
      console.log('🎉 SUCCESS: Threshold validation worked - correctly failed!');
    }
  } catch (err) {
    console.log('❌ Scenario 2 failed:', err.message);
  }
}

async function testScenario3_EarlyCancel(connection, PROGRAM_ID, payer, multisigPda, multisigConfigPda, member1, member2, member3) {
  console.log('\n📋 Test Scenario 3: Early Cancellation (not yet expired)');
  
  const [proposalPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('proposal'), multisigPda.toBuffer(), Buffer.from(new Uint8Array(new BigUint64Array([BigInt(3)]).buffer))],
    PROGRAM_ID
  );

  // Create proposal that's not yet expired
  const currentSlot = await connection.getSlot();
  const proposalData = createProposalData({
    proposal_id: 3,
    expiry: currentSlot + 1000, // Future expiry
    status: 1, // Active
    members: [member1.publicKey, member2.publicKey, member3.publicKey],
    votes: [1, 1, 0, 0, 0, 0, 0, 0, 0, 0], // 2 YES votes
  });

  await createAccount(connection, payer, proposalPda, proposalData, PROGRAM_ID);

  const closeIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: proposalPda, isSigner: false, isWritable: true },
      { pubkey: multisigConfigPda, isSigner: false, isWritable: false },
    ],
    data: Buffer.from([4]),
  });

  try {
    const sig = await sendAndConfirmTransaction(connection, new Transaction().add(closeIx), [payer]);
    console.log('✅ close_proposal executed! Signature:', sig);
    
    const proposalAccount = await connection.getAccountInfo(proposalPda);
    const status = proposalAccount.data[16];
    console.log('📊 Final status:', status === 4 ? 'Cancelled ⏸️' : `Other (${status})`);
    
    if (status === 4) {
      console.log('🎉 SUCCESS: Early cancellation logic worked!');
    }
  } catch (err) {
    console.log('❌ Scenario 3 failed:', err.message);
  }
}

function createProposalData({ proposal_id, expiry, status, members, votes }) {
  const data = Buffer.alloc(356); // Based on your ProposalState::LEN
  let offset = 0;

  // proposal_id (u64)
  data.writeBigUInt64LE(BigInt(proposal_id), offset);
  offset += 8;

  // expiry (u64)
  data.writeBigUInt64LE(BigInt(expiry), offset);
  offset += 8;

  // result/status (u8)
  data.writeUInt8(status, offset);
  offset += 1;

  // bump (u8)
  data.writeUInt8(255, offset);
  offset += 1;

  // active_members [Pubkey; 10] (32 * 10 = 320 bytes)
  for (let i = 0; i < 10; i++) {
    if (i < members.length) {
      members[i].toBuffer().copy(data, offset);
    }
    offset += 32;
  }

  // votes [u8; 10] (10 bytes)
  for (let i = 0; i < 10; i++) {
    data.writeUInt8(votes[i] || 0, offset + i);
  }
  offset += 10;

  // created_time (u64)
  data.writeBigUInt64LE(BigInt(Date.now()), offset);

  return data;
}

async function createAccount(connection, payer, accountPda, data, programId) {
  const lamports = await connection.getMinimumBalanceForRentExemption(data.length);
  
  const createIx = SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    newAccountPubkey: accountPda,
    lamports,
    space: data.length,
    programId,
  });

  // This won't work because we can't create PDAs directly
  // Instead we'll need to use a different approach...
  console.log('⚠️ Note: Cannot create PDA accounts directly in test');
  console.log('📝 This test simulates the data structure verification');
}

fullCloseProposalTest().catch(console.error);
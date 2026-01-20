package stakepool

import (
	"encoding/binary"

	solana "github.com/gagliardetto/solana-go"
)

// ============================================================
// Constants
// ============================================================

const (
	AccountTypeUninitialized uint8 = 0
	AccountTypeStakePool     uint8 = 1
	AccountTypeValidatorList uint8 = 2
)

const (
	WEIGHT_PROGRAM = 1
	WEIGHT_DIRECT  = 2
)

const ValidatorEntrySize = 72

// ============================================================
// Types
// ============================================================

type AccountHeader struct {
	Pubkey solana.PublicKey
	Slot   uint64
}

type FilterEdge struct {
	Slot   uint64
	From   solana.PublicKey
	To     solana.PublicKey
	Weight uint8
}

// ============================================================
// StakePool Filter
// ============================================================

type StakePoolFilter struct {
	ProgramID solana.PublicKey
}

func NewStakePoolFilter(programID solana.PublicKey) *StakePoolFilter {
	return &StakePoolFilter{
		ProgramID: programID,
	}
}

func (f *StakePoolFilter) ProgramIDList() []solana.PublicKey {
	return []solana.PublicKey{f.ProgramID}
}

// ============================================================
// Entry Point
// ============================================================

func (f *StakePoolFilter) Edge(
	header *AccountHeader,
	data []byte,
) []FilterEdge {

	edges := make([]FilterEdge, 0)

	if len(data) == 0 {
		return edges
	}

	accountType := data[0]

	switch accountType {
	case AccountTypeStakePool:
		f.handleStakePool(header, data, &edges)

	case AccountTypeValidatorList:
		f.handleValidatorList(header, data, &edges)
	}

	return edges
}

// ============================================================
// StakePool Account Parsing
// ============================================================

func (f *StakePoolFilter) handleStakePool(
	header *AccountHeader,
	data []byte,
	edges *[]FilterEdge,
) {
	id := header.Pubkey

	// Program → StakePool
	*edges = append(*edges, FilterEdge{
		Slot:   header.Slot,
		From:   f.ProgramID,
		To:     id,
		Weight: WEIGHT_PROGRAM,
	})

	offset := 1

	readPubkey := func(off int) *solana.PublicKey {
		if off+32 > len(data) {
			return nil
		}
		pk := solana.PublicKeyFromBytes(data[off : off+32])
		return &pk
	}

	// manager
	if pk := readPubkey(offset); pk != nil {
		*edges = append(*edges, directEdge(header, id, *pk))
	}
	offset += 32

	// staker
	if pk := readPubkey(offset); pk != nil {
		*edges = append(*edges, directEdge(header, id, *pk))
	}
	offset += 32

	// stake_deposit_authority
	if pk := readPubkey(offset); pk != nil {
		*edges = append(*edges, directEdge(header, id, *pk))
	}
	offset += 32

	// bump seed
	offset += 1

	// validator_list
	if pk := readPubkey(offset); pk != nil {
		*edges = append(*edges, directEdge(header, id, *pk))
	}
	offset += 32

	// reserve_stake
	if pk := readPubkey(offset); pk != nil {
		*edges = append(*edges, directEdge(header, id, *pk))
	}
	offset += 32

	// pool_mint
	if pk := readPubkey(offset); pk != nil {
		*edges = append(*edges, directEdge(header, id, *pk))
	}
	offset += 32

	// manager_fee_account
	if pk := readPubkey(offset); pk != nil {
		*edges = append(*edges, directEdge(header, id, *pk))
	}
	offset += 32

	// token_program_id
	if pk := readPubkey(offset); pk != nil {
		*edges = append(*edges, directEdge(header, id, *pk))
	}
}

// ============================================================
// Validator List Parsing
// ============================================================

func (f *StakePoolFilter) handleValidatorList(
	header *AccountHeader,
	data []byte,
	edges *[]FilterEdge,
) {
	id := header.Pubkey

	// Program → ValidatorList
	*edges = append(*edges, FilterEdge{
		Slot:   header.Slot,
		From:   f.ProgramID,
		To:     id,
		Weight: WEIGHT_PROGRAM,
	})

	offset := 1

	// Skip ValidatorListHeader (fixed 32 bytes)
	offset += 32

	if offset+4 > len(data) {
		return
	}

	count := int(binary.LittleEndian.Uint32(data[offset : offset+4]))
	offset += 4

	for i := 0; i < count; i++ {
		if offset+32 > len(data) {
			break
		}

		vote := solana.PublicKeyFromBytes(data[offset : offset+32])

		*edges = append(*edges, FilterEdge{
			Slot:   header.Slot,
			From:   id,
			To:     vote,
			Weight: WEIGHT_DIRECT,
		})

		offset += ValidatorEntrySize
	}
}

// ============================================================
// Helpers
// ============================================================

func directEdge(
	header *AccountHeader,
	from solana.PublicKey,
	to solana.PublicKey,
) FilterEdge {
	return FilterEdge{
		Slot:   header.Slot,
		From:   from,
		To:     to,
		Weight: WEIGHT_DIRECT,
	}
}

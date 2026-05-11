package server

import "github.com/spf13/cobra"

func NewCreateCmd() *cobra.Command    { return newCreateCmd() }
func NewPsCmd() *cobra.Command        { return newPsCmd() }
func NewInspectCmd() *cobra.Command   { return newInspectCmd() }
func NewRmCmd() *cobra.Command        { return newRmCmd() }
func NewStartCmd() *cobra.Command     { return newStartCmd() }
func NewStopCmd() *cobra.Command      { return newStopCmd() }
func NewRestartCmd() *cobra.Command   { return newRestartCmd() }
func NewUpgradeCmd() *cobra.Command   { return newUpgradeCmd() }
func NewLogsCmd() *cobra.Command      { return newLogsCmd() }
func NewAttachCmd() *cobra.Command    { return newAttachCmd() }
func NewConfigureCmd() *cobra.Command { return newConfigureCmd() }
func NewBackupCmd() *cobra.Command    { return newBackupCmd() }

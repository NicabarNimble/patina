package workspace

// RegistryAdapter makes Workspace compatible with the registry module interface
// This is a temporary adapter until we fully migrate to the modular architecture

func (w *Workspace) GetID() string           { return w.ID }
func (w *Workspace) GetName() string         { return w.Name }
func (w *Workspace) GetStatus() string       { return string(w.Status) }
func (w *Workspace) GetBranchName() string   { return w.BranchName }
func (w *Workspace) GetWorktreePath() string { return w.WorktreePath }
func (w *Workspace) GetBaseImage() string    { return w.Config.BaseImage }
func (w *Workspace) GetCreatedAt() string    { return w.CreatedAt.Format("2006-01-02T15:04:05Z07:00") }
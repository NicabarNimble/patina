// Package registry is a patch of patina - a thin, durable interface
// that queries active development environments without modifying them.
//
// Like oxidized metal forming a protective layer, this module provides
// a stable read-only view into the workspace ecosystem. It extracts just
// the registry operations from the larger workspace system, following the
// "tools not systems" principle.
//
// This is an Eternal Tool pattern - its API can remain unchanged for years
// while the underlying workspace implementation evolves.
package registry
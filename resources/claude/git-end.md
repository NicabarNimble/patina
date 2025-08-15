# /git-end

End Git work tracking and analyze survival patterns.

## Usage

```
/git-end
```

## Description

Concludes Git work tracking with comprehensive analysis:
- Work summary with commit and file statistics
- Survival analysis of modified code
- Co-modification pattern detection
- Experimental indicators (uncommitted changes, test modifications)
- Interactive classification (feature/bugfix/refactor/experiment/research)
- Outcome tracking (completed/partial/failed/ongoing)
- Special handling for failed experiments (option to create exp/ branch)
- Archives work history for future memory

## Philosophy

**Failed Experiments are Gold**: Preserves failed attempts as valuable memory. Offers to create experimental branches for failed work to prevent repeating mistakes.

## Classification Types

- **feature**: New functionality added
- **bugfix**: Problem solved
- **refactor**: Code improved without changing behavior  
- **experiment**: Trying something (might fail)
- **research**: Learning/exploring code

## Outcomes

- **completed**: Work is done
- **partial**: Some progress made
- **failed**: Didn't work out (valuable for memory!)
- **ongoing**: Will continue later

## Examples

```
/git-end
# Interactive prompts will guide classification
```

Archives to `.claude/context/git-work/archive/` for future reference.

## Related Commands

- `/git-start` - Begin new Git work
- `/git-update` - Track progress
- `/git-note` - Capture insights
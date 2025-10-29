# Testing GitHub Actions Locally with `act`

This guide shows you how to test the GitHub Actions workflow locally before pushing to GitHub.

## Prerequisites

✅ **Docker** - Required to run containers
✅ **act** - GitHub Actions local runner

Both are already installed on your system!

---

## Quick Start

### 1. List Available Workflows

```bash
act -l
```

This shows all jobs in your workflows.

### 2. Test Specific Jobs

**Run just the test job:**
```bash
act -j test
```

**Run just the lint job:**
```bash
act -j lint
```

**Run the build job (single platform):**
```bash
act -j build --matrix platform:ubuntu-22.04
```

### 3. Dry Run (See What Would Execute)

```bash
act -n
```

This shows what would run without actually executing.

---

## Platform-Specific Testing

### Test Linux Build (Easiest)

```bash
act -j build --matrix platform:ubuntu-22.04
```

### Test macOS/Windows (Simulated)

Since `act` runs in Docker (Linux containers), macOS and Windows builds are simulated:

```bash
# Simulate macOS
act -j build --matrix platform:macos-latest

# Simulate Windows
act -j build --matrix platform:windows-latest
```

**Note:** These won't produce actual .dmg or .exe files, but will test:
- ✅ Workflow syntax
- ✅ Job dependencies
- ✅ Environment variables
- ✅ Script execution
- ✅ Most build steps

---

## Full Workflow Testing

### Run Everything (All Jobs)

```bash
act push
```

This simulates a `push` event and runs all jobs.

### Run Pull Request Workflow

```bash
act pull_request
```

---

## Advanced Options

### Run with Verbose Output

```bash
act -v -j test
```

### Run with Secrets

```bash
act -j build --secret GITHUB_TOKEN=your_token_here
```

Or create `.secrets` file:
```bash
echo "GITHUB_TOKEN=ghp_xxxxxxxxxxxx" > .secrets
act -j build --secret-file .secrets
```

### Use Different Docker Images

**Small images (faster, less compatible):**
```bash
act -P ubuntu-latest=node:16-bullseye-slim
```

**Large images (slower, more compatible):**
```bash
act -P ubuntu-latest=catthehacker/ubuntu:full-latest
```

---

## Recommended Testing Workflow

### Before Pushing to GitHub:

1. **Test syntax and structure:**
   ```bash
   act -n
   ```

2. **Test individual jobs:**
   ```bash
   act -j test
   act -j lint
   ```

3. **Test Linux build (most realistic):**
   ```bash
   act -j build --matrix platform:ubuntu-22.04
   ```

4. **If all passes, push to GitHub** for full platform testing

---

## Limitations of Local Testing

### ❌ Won't Work Locally:
- **macOS .dmg/.app builds** - Requires actual macOS
- **Windows .exe/.msi builds** - Requires actual Windows
- **Code signing** - Requires certificates
- **Artifact uploads to GitHub** - No actual GitHub context

### ✅ Will Work Locally:
- **Workflow syntax validation**
- **Job execution order**
- **Script execution**
- **Environment variables**
- **Dependency installation (pnpm, etc.)**
- **Tests and linting**
- **Most build steps**

---

## Common Issues & Solutions

### Issue: "Permission denied" on Docker socket

**Solution:**
```bash
sudo chmod 666 /var/run/docker.sock
# Or add yourself to docker group:
sudo usermod -aG docker $USER
```

### Issue: "Container exits immediately"

**Solution:** Use verbose mode to see errors:
```bash
act -v -j test
```

### Issue: "Out of disk space"

**Solution:** Clean up Docker:
```bash
docker system prune -a
```

### Issue: "Can't find .actrc"

**Solution:** Config is in repo root at `.actrc` (already created!)

---

## Performance Tips

### Speed Up Runs

1. **Use `.actrc` (already configured)** - Reuses containers
2. **Test specific jobs** - Don't run everything
3. **Use smaller images** when possible
4. **Cache dependencies:**
   ```bash
   act --use-gitignore=false
   ```

### Parallel Testing

```bash
# Run multiple jobs in parallel
act -j test & act -j lint &
```

---

## Real-World Workflow

```bash
# 1. Make changes to workflow
vim .github/workflows/build.yml

# 2. Quick syntax check
act -n

# 3. Test what you changed
act -j test

# 4. Full test (if needed)
act push

# 5. Push to GitHub for full platform builds
git push
```

---

## Useful Commands Reference

| Command | Description |
|---------|-------------|
| `act -l` | List all jobs |
| `act -n` | Dry run (no execution) |
| `act -j <job>` | Run specific job |
| `act -v` | Verbose output |
| `act push` | Simulate push event |
| `act pull_request` | Simulate PR event |
| `act --matrix <key:value>` | Run specific matrix item |
| `act --reuse` | Reuse containers (faster) |
| `act --rm` | Remove containers after run |

---

## Documentation

- **act GitHub**: https://github.com/nektos/act
- **act Documentation**: https://nektosact.com/
- **Docker Images**: https://github.com/catthehacker/docker_images

---

## TL;DR - Quick Test

```bash
# Test the workflow quickly:
act -j test -j lint

# Test Linux build:
act -j build --matrix platform:ubuntu-22.04

# Push to GitHub for full cross-platform builds
```

**Remember:** `act` is great for quick validation, but full cross-platform builds still need GitHub Actions runners! 🚀

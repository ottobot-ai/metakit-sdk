# Metakit SDK Publishing Guide

Complete steps to finish setup and publish the SDK packages to npm and PyPI.

## Prerequisites

- Admin access to the `Constellation-Labs/metakit-sdk` GitHub repository
- npm account with access to the `@constellation-network` organization
- PyPI account with access to publish `constellation-metagraph-sdk`

---

## Step 1: Commit and Push Initial Code

```bash
cd /home/scas/git/metakit-sdk

# Stage all files
git add .

# Commit
git commit -m "Initial monorepo setup with TypeScript and Python SDKs

- TypeScript SDK: @constellation-network/metagraph-sdk
- Python SDK: constellation-metagraph-sdk
- CI/CD with GitHub Actions
- Cross-language test vectors
- Dependabot configuration"

# Push to main
git push origin main
```

---

## Step 2: Verify CI Passes

1. Go to https://github.com/Constellation-Labs/metakit-sdk/actions
2. Wait for the CI workflow to complete
3. Ensure all jobs pass:
   - TypeScript tests (Node 18, 20, 22)
   - Python tests (Python 3.10, 3.11, 3.12)
   - Cross-language compatibility tests

**If CI fails**: Fix the issues and push again before proceeding.

---

## Step 3: Configure npm Publishing

### 3.1 Create npm Access Token

1. Log in to https://www.npmjs.com
2. Go to **Access Tokens** → **Generate New Token**
3. Select **Automation** token type
4. Name it `metakit-sdk-github-actions`
5. Copy the token (starts with `npm_`)

### 3.2 Add Token to GitHub Secrets

1. Go to https://github.com/Constellation-Labs/metakit-sdk/settings/secrets/actions
2. Click **New repository secret**
3. Name: `NPM_TOKEN`
4. Value: Paste the npm token
5. Click **Add secret**

---

## Step 4: Configure PyPI Trusted Publishing

PyPI trusted publishing uses OpenID Connect (OIDC) - no secrets needed!

### 4.1 Add Trusted Publisher on PyPI

1. Log in to https://pypi.org
2. If project doesn't exist yet, create a "pending publisher" at https://pypi.org/manage/account/publishing/
3. For the project `constellation-metagraph-sdk`:
   - Go to **Manage** → **Publishing**
   - Under **Trusted Publishers**, click **Add a new publisher**
4. Fill in:
   - **Owner**: `Constellation-Labs`
   - **Repository name**: `metakit-sdk`
   - **Workflow name**: `release-python.yml`
   - **Environment name**: (leave blank)
5. Click **Add**

---

## Step 5: Release TypeScript SDK

### 5.1 Verify Version

Ensure `packages/typescript/package.json` has the correct version:

```bash
grep '"version"' packages/typescript/package.json
```

### 5.2 Create and Push Tag

```bash
# Create annotated tag
git tag -a typescript-v0.1.0 -m "TypeScript SDK v0.1.0"

# Push tag to trigger release
git push origin typescript-v0.1.0
```

### 5.3 Monitor Release

1. Watch the workflow at https://github.com/Constellation-Labs/metakit-sdk/actions
2. Once complete, verify:
   - GitHub Release created: https://github.com/Constellation-Labs/metakit-sdk/releases
   - npm package published: https://www.npmjs.com/package/@constellation-network/metagraph-sdk

---

## Step 6: Release Python SDK

### 6.1 Verify Version

Ensure `packages/python/pyproject.toml` has the correct version:

```bash
grep 'version' packages/python/pyproject.toml | head -1
```

### 6.2 Create and Push Tag

```bash
# Create annotated tag
git tag -a python-v0.1.0 -m "Python SDK v0.1.0"

# Push tag to trigger release
git push origin python-v0.1.0
```

### 6.3 Monitor Release

1. Watch the workflow at https://github.com/Constellation-Labs/metakit-sdk/actions
2. Once complete, verify:
   - GitHub Release created: https://github.com/Constellation-Labs/metakit-sdk/releases
   - PyPI package published: https://pypi.org/project/constellation-metagraph-sdk/

---

## Step 7: Post-Release Verification

### Test npm Installation

```bash
mkdir /tmp/test-ts-sdk && cd /tmp/test-ts-sdk
npm init -y
npm install @constellation-network/metagraph-sdk @stardust-collective/dag4

node -e "
const { generateKeyPair } = require('@constellation-network/metagraph-sdk');
const kp = generateKeyPair();
console.log('Generated address:', kp.address);
"
```

### Test PyPI Installation

```bash
mkdir /tmp/test-py-sdk && cd /tmp/test-py-sdk
python3 -m venv venv && source venv/bin/activate
pip install constellation-metagraph-sdk

python3 -c "
from constellation_sdk import generate_key_pair
kp = generate_key_pair()
print('Generated address:', kp.address)
"
```

---

## Future Releases

To release a new version:

1. **Update version** in package config:
   - TypeScript: `packages/typescript/package.json`
   - Python: `packages/python/pyproject.toml`

2. **Commit and push**:
   ```bash
   git add .
   git commit -m "chore: bump version to X.Y.Z"
   git push origin main
   ```

3. **Create and push tag**:
   ```bash
   # TypeScript
   git tag -a typescript-vX.Y.Z -m "TypeScript SDK vX.Y.Z"
   git push origin typescript-vX.Y.Z

   # Python
   git tag -a python-vX.Y.Z -m "Python SDK vX.Y.Z"
   git push origin python-vX.Y.Z
   ```

4. **Verify** the release workflow completes successfully

---

## Troubleshooting

### npm publish fails with 403

- Verify `NPM_TOKEN` secret is set correctly
- Verify token has publish permissions
- Verify account has access to `@constellation-network` org

### PyPI publish fails with "trusted publisher not found"

- Ensure trusted publisher is configured exactly as:
  - Owner: `Constellation-Labs`
  - Repository: `metakit-sdk`
  - Workflow: `release-python.yml`
- If package doesn't exist, create a pending publisher first

### Version mismatch error

The release workflows verify that the package version matches the tag. If you see:
```
Package version (X.X.X) does not match tag (Y.Y.Y)
```

Update the version in the package config file, commit, and push before creating the tag.

### Manual workflow run

You can manually trigger the release workflows from the Actions tab using "Run workflow". This is useful for testing but won't publish (publish steps only run on tag push).

---

## Quick Reference

| Action | TypeScript | Python |
|--------|------------|--------|
| Version file | `packages/typescript/package.json` | `packages/python/pyproject.toml` |
| Tag format | `typescript-v0.1.0` | `python-v0.1.0` |
| npm package | `@constellation-network/metagraph-sdk` | - |
| PyPI package | - | `constellation-metagraph-sdk` |
| Workflow | `release-typescript.yml` | `release-python.yml` |
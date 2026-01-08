# Contributing to Constellation Metagraph SDK

Thank you for your interest in contributing to the Constellation Metagraph SDK!

## Development Setup

### Prerequisites

- Node.js 18+
- Python 3.10+
- Git

### Getting Started

1. Fork and clone the repository:
   ```bash
   git clone https://github.com/YOUR_USERNAME/metakit-sdk.git
   cd metakit-sdk
   ```

2. Install dependencies:
   ```bash
   # TypeScript
   npm install

   # Python
   cd packages/python
   python3 -m venv venv
   source venv/bin/activate
   pip install -e ".[dev]"
   cd ../..
   ```

3. Run tests to verify setup:
   ```bash
   npm run test:all
   ```

## Development Workflow

### Making Changes

1. Create a feature branch:
   ```bash
   git checkout -b feature/my-feature
   ```

2. Make your changes

3. Run tests and linting:
   ```bash
   npm run test:all
   npm run lint:all
   ```

4. Commit your changes with a descriptive message

5. Push and create a pull request

### Code Style

#### TypeScript
- Use Prettier for formatting (`npm run format:ts`)
- Use ESLint for linting (`npm run lint:ts`)
- Write tests for new functionality

#### Python
- Use Black for formatting (`npm run format:py`)
- Use isort for import sorting
- Use Ruff for linting (`npm run lint:py`)
- Use mypy for type checking
- Write tests for new functionality

## Testing

### Running Tests

```bash
# All tests
npm run test:all

# TypeScript only
npm run test:ts

# Python only
npm run test:py

# Cross-language compatibility
npm test -w packages/typescript -- --testPathPattern=cross-language
cd packages/python && pytest tests/test_cross_language.py
```

### Cross-Language Compatibility

When modifying signing or hashing logic, ensure that:

1. All test vectors in `/shared/test_vectors.json` still pass
2. Both TypeScript and Python implementations produce identical results
3. Signatures are interoperable between languages

## Pull Request Guidelines

1. **Title**: Use a clear, descriptive title
2. **Description**: Explain what changes you made and why
3. **Tests**: Include tests for new functionality
4. **Documentation**: Update relevant documentation
5. **Changelog**: Add an entry to the package's CHANGELOG.md

## Reporting Issues

When reporting issues, please include:

1. SDK version and language (TypeScript/Python)
2. Node.js/Python version
3. Operating system
4. Steps to reproduce
5. Expected vs actual behavior
6. Any error messages or stack traces

## Release Process

Releases are managed by maintainers. The process is:

1. Update version in package config
2. Update CHANGELOG.md
3. Create PR and merge to main
4. Create GitHub Release with appropriate tag:
   - TypeScript: `typescript-v1.2.3`
   - Python: `python-v1.2.3`
5. GitHub Actions automatically publishes to npm/PyPI

## Questions?

If you have questions, feel free to:
- Open a GitHub issue
- Reach out on the Constellation Network Discord

## License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 License.
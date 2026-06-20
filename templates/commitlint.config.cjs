# Commitlint Configuration Template
# Copy this to commitlint.config.cjs in your project
module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    // Type must be one of the allowed types
    'type-enum': [
      2,
      'always',
      [
        'feat',     // New feature
        'fix',      // Bug fix
        'docs',     // Documentation only changes
        'style',    // Code style (formatting, missing semi-colons, etc)
        'refactor', // Code change that neither fixes a bug nor adds a feature
        'perf',     // Performance improvement
        'test',     // Adding missing tests
        'chore',    // Build process or auxiliary tool changes
        'ci',       // CI configuration changes
        'revert',   // Revert a previous commit
      ],
    ],
    // Subject must not be empty
    'subject-empty': [2, 'never'],
    // Type must not be empty
    'type-empty': [2, 'never'],
    // Subject case is not checked (allow any case)
    'subject-case': [0],
    // Header max length (type(scope): subject)
    'header-max-length': [2, 'always', 100],
  },
};

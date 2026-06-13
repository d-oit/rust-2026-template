module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    // Enforce 100-char body/footer line length (matches CI rule)
    'body-max-line-length': [2, 'always', 100],
    'footer-max-line-length': [2, 'always', 100],
  },
  ignores: [
    (message) => message.includes('Co-authored-by: codacy-production[bot]'),
    (message) => /^Update .*/.test(message) && message.includes('bot'),
    (message) => message.includes('---\nupdated-dependencies:'),
  ],
};

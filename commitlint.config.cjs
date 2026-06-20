module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'body-max-line-length': [0],
  },
  ignores: [
    (message) => message.includes('Co-authored-by: codacy-production[bot]'),
    (message) => /^Update .*/.test(message) && message.includes('bot'),
    (message) => message.includes('---\nupdated-dependencies:'),
    (message) => /^Merge branch .+ into/.test(message),
    (message) => /^Merge pull request/.test(message),
  ],
};

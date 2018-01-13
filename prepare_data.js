const fs = require('fs');

const content = fs.readFileSync('foo.txt', 'utf-8');

fs.writeFileSync(
  'foo.txt',
  content
    .split('.')
    .map(l => l.trim())
    .join('.\n')
);

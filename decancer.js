const constants = require('./index.json');
const alphabet = [...'abcdefghijklmnopqrstuvwxyz'];
const startRegex = new RegExp(constants.startRegex, 'g');

/**
 * @param {string} text The text to decancer.
 * @returns {string} The cleaned string. Will ALWAYS be in lowercase.
 */
module.exports = (text) => {
    if (typeof text !== 'string' || !text.length)
        throw new TypeError("'text' must be a string and it must contain at least a character.");

    else if (!/[^\u0000-\u007F]/.test(text))
        return text.toLowerCase(); // sorry
    
    for (const [k, v] of Object.entries(constants.emojis))
        text = text.replace(new RegExp(v, 'g'), k);

    for (const [k, v] of Object.entries(constants.others))
        text = text.replace(new RegExp(`[${v}]`), k);
    
    text = text
      .toLowerCase()
      .replace(startRegex, '');
    
    for (let i = 0; i < 26; i++) {
        const [ styles, extras ] = constants.alphabetical[i].split(';');
        text = text
          .replace(new RegExp(`[${extras}]`, 'gi'), alphabet[i])
          .replace(new RegExp([...styles].map(x => `(${x})`).join('|'), 'g'), alphabet[i]);
    }

    return text.replace(/[\uD800-\uDB7F]/g, '');
};
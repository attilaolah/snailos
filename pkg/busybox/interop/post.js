console.log("post.js");

ENV["USER"] = Module.user;
FS.rename("/home/web_user", `/home/${Module.user}`);

const _get_char = FS_stdin_getChar;
FS_stdin_getChar = () => {
  console.log("TODO: get_char()!");
  return _get_char();
};

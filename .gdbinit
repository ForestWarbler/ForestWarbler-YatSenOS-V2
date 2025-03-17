# ni10 命令：单步执行后显示当前10条汇编指令
define ni10
    ni
    x/10i $pc
end

# 定义 remoteorb 命令：连接到远程主机
define remoteorb
    gef-remote host.orb.internal 1234
end


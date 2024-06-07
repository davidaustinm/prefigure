# here are some calculus operations for developers
# TODO:  should probably rename and include more math

def derivative(f, a):
    return richardson(f, a, 0.1, 4)

def richardson(f, a, h, k):
    E = []
    for i in range(k):
        delta = h/float(2**i)
        E.append((f(a+delta) - f(a))/delta)

    j = 1
    while len(E) > 1:
        nextE = []
        for i in range(len(E) - 1):
            nextE.append(E[i+1] + (E[i+1] - E[i])/(2**j-1))
        E = nextE
        j += 1
    return E[0]

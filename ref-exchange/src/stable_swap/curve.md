
# math for stable swap pool
## About the D
---

### The equation: 

$$ 
An^n \sum x_{i} + D = ADn^n + \frac{D^{n+1}}{n^n \prod x_{i}} 
$$

$$
f(D) = \frac{D^{n+1}}{n^n \prod x_{i}} + (An^n - 1)D - An^n \sum x_{i} = 0
$$

$$
f'(D) = \frac{n+1}{n^n \prod x_{i}} D^n + An^n - 1 = 0
$$

### Solve for D using Newton method
---
  
Newton's method to solve for D:  
$$
D_{k+1} = D_{k} - \frac{f(D_{k})}{f'(D_{k})}
$$


Let  
$$
D_{prod} = \frac{D^{n+1}}{n^n \prod x_{i}} 
$$
  
Then,  
$$
D_{k+1} = \frac{D_k(An^n \sum x_{i} + nD_{k,prod})} {D_{k}(An^n - 1) + (n+1)D_{k,prod}}
$$  

### Specialize 
---
Our conditions:  

$ n = 2 $
     
$ \sum x_i = (x+y) $  
  
$ \prod x_i = (xy) $  
  
So,   
$$
D_{prod} = \frac{D^{3}}{4xy} 
$$
  
$$
D_{k+1} = \frac{D_k(4A(x+y) + 2D_{k,prod})} {D_{k}(4A - 1) + 3D_{k,prod}}
$$
  
## About the y
---
Let's withdraw y from $x_i$ :  
$$
\sum x_i = y + \sum x'_i  
$$
  
$$
\prod x_i = y \prod x'_i
$$

Assume we know $D$, $A$ and $x_i$, let's solve for $y$:

$$ 
An^n (y+\sum x'_{i}) + D = ADn^n + \frac{D^{n+1}}{n^n y \prod x'_i} 
$$  
Make it to be $f(y)$:  
$$
An^ny^2 + [An^n\sum x'_i - (An^n-1)D]y = \frac{D^{n+1}}{n^n\prod x'_i}
$$  

Simplify to $f(y)$ is:  
$$
y^2 + (\sum x'_i + \frac{D}{An^n} - D)y = \frac{D^{n+1}}{An^{2n}\prod x'_i}
$$  

And $f'(y)$ is:  
$$
2y + \sum x'_i + \frac{D}{An^n} - D = 0
$$

### Solve for y using Newton method
---
  
Newton's method to solve for D:  

$$
y_{k+1} = y_{k} - \frac{f(y_{k})}{f'(y_{k})}
$$  

Let's define $b$, $c$ as:  

$$
b = \sum x'_i + \frac{D}{An^n}
$$
  
$$
c = \frac{D^{n+1}}{An^{2n}\prod x'_i}
$$  

Then:  
$$
y_{k+1} = \frac{y_k^2 + c}{2y_k + b - D}
$$  